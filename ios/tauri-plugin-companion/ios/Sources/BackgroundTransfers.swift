import Foundation
import CryptoKit
import Security
import MusicOSBackgroundHooks

struct BackgroundAsset: Codable {
    var hash: String
    var path: String
    var kind: String
    var size: Int64?
    var fingerprint: String
}

// URLSession owns network work while the webview is suspended. Only verified assets are published.
@MainActor final class BackgroundTransfers: NSObject, URLSessionDownloadDelegate, URLSessionTaskDelegate {
    static let shared = BackgroundTransfers()
    static let identifier = "com.musicos.companion.transfers.v1"
    let root: URL
    var approvedPin:(()->(host:String,fingerprint:String)?)?
    private var activeTasks:[String:Int]=[:]
    private var cancellationEpoch=0
    var ready: (() -> Void)?
    var progress: ((Int,Int) -> Void)?
    private var waiters: [String: [CheckedContinuation<URL, Error>]] = [:]
    private var results: [String: Result<URL, Error>] = [:]
    private var planned = Set<String>()
    private var completed = Set<String>()
    private var verifying = 0
    private var eventsFinished = false
    private var retainResume = true
    private var completionHandlers: [() -> Void] = []
    private lazy var session: URLSession = {
        let config = URLSessionConfiguration.background(withIdentifier: Self.identifier)
        config.sessionSendsLaunchEvents = true
        config.isDiscretionary = false
        config.allowsCellularAccess = false
        config.waitsForConnectivity = true
        config.httpMaximumConnectionsPerHost = 2
        config.timeoutIntervalForRequest = 60
        config.timeoutIntervalForResource = 7*24*3600
        return URLSession(configuration: config, delegate: self, delegateQueue: .main)
    }()
    override init() {
        root = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("musicOS")
        super.init()
        MusicOSSetBackgroundCallback { identifier, completion in
            Task { @MainActor in
                guard identifier == Self.identifier else { completion(); return }
                Self.shared.completionHandlers.append(completion)
                _ = Self.shared.session
                Self.shared.finishEventsIfReady()
            }
        }
        _ = session
    }
    func destination(_ asset: BackgroundAsset) -> URL { root.appendingPathComponent("\(asset.kind)/\(asset.hash)") }
    func start(_ assets: [BackgroundAsset], base: URL, authorization: String) async {
        let existing = await session.allTasks
        guard !Task.isCancelled else { return }
        retainResume=true
        results.removeAll()
        activeTasks=Dictionary(existing.compactMap{task in decode(task).map{($0.hash,task.taskIdentifier)}},uniquingKeysWith:{a,_ in a})
        let active=Set(activeTasks.keys)
        planned = Set(assets.map(\.hash)); completed = []
        for asset in assets {
            if FileManager.default.fileExists(atPath: destination(asset).path) { completed.insert(asset.hash); continue }
            guard !active.contains(asset.hash) else { continue }
            results.removeValue(forKey:asset.hash)
            var request = URLRequest(url: base.appendingPathComponent(asset.path))
            request.setValue(authorization, forHTTPHeaderField:"Authorization")
            request.setValue("\"\(asset.hash)\"", forHTTPHeaderField:"If-Match")
            let task: URLSessionDownloadTask
            if let data = resumeData(asset.hash,endpoint:request.url!.absoluteString,fingerprint:asset.fingerprint) { task=session.downloadTask(withResumeData:data) }
            else { task=session.downloadTask(with:request) }
            task.taskDescription=String(data:try! JSONEncoder().encode(asset),encoding:.utf8)
            activeTasks[asset.hash]=task.taskIdentifier
            task.resume()
        }
        progress?(completed.count,planned.count)
    }
    func value(_ asset: BackgroundAsset) async throws -> URL {
        let file=destination(asset)
        if FileManager.default.fileExists(atPath:file.path) {return file}
        if let result=results[asset.hash] {return try result.get()}
        return try await withCheckedThrowingContinuation { waiters[asset.hash,default:[]].append($0) }
    }
    func cancel(keepResume:Bool=true) async {
        retainResume=keepResume;cancellationEpoch+=1;let epoch=cancellationEpoch
        activeTasks.removeAll()
        if !keepResume{SecItemDelete([kSecClass as String:kSecClassGenericPassword,kSecAttrService as String:"com.musicos.companion.resume"] as CFDictionary)}
        for task in await session.allTasks {
            if let download=task as? URLSessionDownloadTask,let asset=decode(task) {
                download.cancel {data in Task{@MainActor in if keepResume,self.retainResume,self.cancellationEpoch==epoch,let data=data {Self.storeResume(data,hash:asset.hash,endpoint:download.originalRequest?.url?.absoluteString ?? "",fingerprint:asset.fingerprint)}}}
            } else {task.cancel()}
        }
        for hash in Array(waiters.keys) {finish(hash,.failure(CancellationError()))}
    }
    private func decode(_ task: URLSessionTask) -> BackgroundAsset? {
        task.taskDescription?.data(using:.utf8).flatMap{try? JSONDecoder().decode(BackgroundAsset.self,from:$0)}
    }
    private func finish(_ hash: String,_ result: Result<URL,Error>) {
        results[hash]=result
        if case let .failure(error)=result,[401,403,409,412].contains((error as NSError).code){Self.storeResume(nil,hash:hash)}
        if case .success = result {completed.insert(hash);Self.storeResume(nil,hash:hash);ready?()}
        let list=waiters.removeValue(forKey:hash) ?? [];for waiter in list{waiter.resume(with:result)}
        progress?(completed.count,planned.count)
    }
    nonisolated func urlSession(_ session:URLSession,downloadTask:URLSessionDownloadTask,didFinishDownloadingTo location:URL) {
        guard let description=downloadTask.taskDescription?.data(using:.utf8),let asset=try? JSONDecoder().decode(BackgroundAsset.self,from:description),["audio","artwork","lyrics"].contains(asset.kind),asset.hash.count==64,asset.hash.allSatisfy(\.isHexDigit) else{return}
        let stage=FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        do {try FileManager.default.moveItem(at:location,to:stage)}catch{Task{@MainActor in self.finish(asset.hash,.failure(error))};return}
        let http=downloadTask.response as? HTTPURLResponse
        Task{@MainActor in
            guard activeTasks[asset.hash]==downloadTask.taskIdentifier else{try? FileManager.default.removeItem(at:stage);return}
            verifying+=1
            let result=await Task.detached(priority:.utility){()->Result<URL,Error> in
                do{
                    guard let http=http,(200...299).contains(http.statusCode)else{throw NSError(domain:"musicOS",code:http?.statusCode ?? 0,userInfo:[NSLocalizedDescriptionKey:"The computer rejected a background transfer. Sync again to refresh its connection."])}
                    let input=try FileHandle(forReadingFrom:stage);defer{try? input.close()};var hash=SHA256()
                    while let data=try input.read(upToCount:1<<20),!data.isEmpty{hash.update(data:data)}
                    let digest=hash.finalize().map{String(format:"%02x",$0)}.joined()
                    guard digest==asset.hash else{throw NSError(domain:"musicOS",code:2,userInfo:[NSLocalizedDescriptionKey:"A background copy failed verification. Retry sync to copy it again."])}
                    let root=FileManager.default.urls(for:.applicationSupportDirectory,in:.userDomainMask)[0].appendingPathComponent("musicOS/\(asset.kind)")
                    try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
                    let target=root.appendingPathComponent(asset.hash)
                    if !FileManager.default.fileExists(atPath:target.path){try FileManager.default.moveItem(at:stage,to:target)}else{try? FileManager.default.removeItem(at:stage)}
                    try? FileManager.default.setAttributes([.protectionKey:FileProtectionType.completeUntilFirstUserAuthentication],ofItemAtPath:target.path)
                    return .success(target)
                }catch{try? FileManager.default.removeItem(at:stage);return .failure(error)}
            }.value
            if activeTasks[asset.hash]==downloadTask.taskIdentifier{activeTasks.removeValue(forKey:asset.hash);finish(asset.hash,result)};verifying-=1;finishEventsIfReady()
        }
    }
    nonisolated func urlSession(_ session:URLSession,task:URLSessionTask,didCompleteWithError error:Error?){
        guard let error=error,let description=task.taskDescription?.data(using:.utf8),let asset=try? JSONDecoder().decode(BackgroundAsset.self,from:description)else{return}
        Task{@MainActor in
            guard self.activeTasks[asset.hash]==task.taskIdentifier else{return}
            if self.retainResume,let data=(error as NSError).userInfo[NSURLSessionDownloadTaskResumeData] as? Data{Self.storeResume(data,hash:asset.hash,endpoint:task.originalRequest?.url?.absoluteString ?? "",fingerprint:asset.fingerprint)}
            self.activeTasks.removeValue(forKey:asset.hash);self.finish(asset.hash,.failure(error))
        }
    }
    nonisolated func urlSession(_ session:URLSession,didReceive challenge:URLAuthenticationChallenge,completionHandler:@escaping(URLSession.AuthChallengeDisposition,URLCredential?)->Void){
        Task{@MainActor in
            guard let pin=self.approvedPin?(),challenge.protectionSpace.host.lowercased()==pin.host.lowercased(),challenge.protectionSpace.authenticationMethod==NSURLAuthenticationMethodServerTrust,let trust=challenge.protectionSpace.serverTrust,let cert=SecTrustGetCertificateAtIndex(trust,0)else{completionHandler(.cancelAuthenticationChallenge,nil);return}
            let digest=SHA256.hash(data:SecCertificateCopyData(cert) as Data).map{String(format:"%02x",$0)}.joined()
            guard digest==pin.fingerprint.lowercased()else{completionHandler(.cancelAuthenticationChallenge,nil);return};completionHandler(.useCredential,URLCredential(trust:trust))
        }
    }

    nonisolated func urlSession(_ session:URLSession,task:URLSessionTask,didReceive challenge:URLAuthenticationChallenge,completionHandler:@escaping(URLSession.AuthChallengeDisposition,URLCredential?)->Void){
        guard let description=task.taskDescription?.data(using:.utf8),let asset=try? JSONDecoder().decode(BackgroundAsset.self,from:description),challenge.protectionSpace.authenticationMethod==NSURLAuthenticationMethodServerTrust,let trust=challenge.protectionSpace.serverTrust,let cert=SecTrustGetCertificateAtIndex(trust,0)else{completionHandler(.cancelAuthenticationChallenge,nil);return}
        let hash=SHA256.hash(data:SecCertificateCopyData(cert) as Data).map{String(format:"%02x",$0)}.joined()
        guard hash==asset.fingerprint.lowercased()else{completionHandler(.cancelAuthenticationChallenge,nil);return};completionHandler(.useCredential,URLCredential(trust:trust))
    }
    nonisolated func urlSession(_ session:URLSession,task:URLSessionTask,willPerformHTTPRedirection response:HTTPURLResponse,newRequest request:URLRequest,completionHandler:@escaping(URLRequest?)->Void){completionHandler(nil)}
    nonisolated func urlSessionDidFinishEvents(forBackgroundURLSession session:URLSession){Task{@MainActor in self.eventsFinished=true;self.finishEventsIfReady()}}
    private func finishEventsIfReady(){guard eventsFinished,verifying==0 else{return};let callbacks=completionHandlers;completionHandlers=[];eventsFinished=false;for callback in callbacks{callback()}}
    private func resumeData(_ hash:String,endpoint:String,fingerprint:String)->Data? {
        var value:CFTypeRef?
        let query:[String:Any]=[kSecClass as String:kSecClassGenericPassword,kSecAttrService as String:"com.musicos.companion.resume",kSecAttrAccount as String:hash,kSecReturnData as String:true]
        guard SecItemCopyMatching(query as CFDictionary,&value)==errSecSuccess,let data=value as? Data,let row=try? JSONSerialization.jsonObject(with:data) as? [String:String],row["endpoint"]==endpoint,row["fingerprint"]==fingerprint,let payload=row["data"]else{return nil}
        return Data(base64Encoded:payload)
    }
    nonisolated private static func storeResume(_ data:Data?,hash:String,endpoint:String="",fingerprint:String=""){
        let query:[String:Any]=[kSecClass as String:kSecClassGenericPassword,kSecAttrService as String:"com.musicos.companion.resume",kSecAttrAccount as String:hash]
        SecItemDelete(query as CFDictionary)
        if let data=data,let envelope=try? JSONSerialization.data(withJSONObject:["data":data.base64EncodedString(),"endpoint":endpoint,"fingerprint":fingerprint]){
            var attributes=query;attributes[kSecValueData as String]=envelope;attributes[kSecAttrAccessible as String]=kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly;SecItemAdd(attributes as CFDictionary,nil)
        }
    }
}
