import Tauri
import UIKit
import AVFoundation
import MediaPlayer
import AVKit
import WebKit
import CryptoKit
import Security
import MediaAccessibility

@_silgen_name("musicos_prepare_audio")
private func prepareAudio(_ source: UnsafePointer<CChar>, _ destination: UnsafePointer<CChar>) -> Int32

@_silgen_name("musicos_read_audio_info")
private func readAudioInfo(_ source:UnsafePointer<CChar>,_ output:UnsafeMutablePointer<CChar>,_ capacity:Int)->Int32

@_silgen_name("musicos_artwork_colors")
private func readArtworkColors(_ source:UnsafePointer<CChar>,_ output:UnsafeMutablePointer<CChar>,_ capacity:Int)->Int32

private struct Connection: Codable {
    var host: String; var port: Int; var server: String; var fingerprint: String
    var client: String; var token: String; var syncChanges:Bool?
    var base: URL { URL(string: "https://\(host.contains(":") ? "[\(host)]" : host):\(port)/v1")! }
}
private enum CompanionError: LocalizedError {
    case message(String)
    case server(Int,String)
    var errorDescription:String? {
        switch self {
        case let .message(message):return message
        case let .server(code,details):
            if code==401 || code==403{return "This computer hasn’t approved the iPhone. Approve it in musicOS Devices, or create a new pairing code."}
            if code==422{return "The computer and iPhone use different sync versions. Update musicOS on the computer, then pair again."}
            if code==503{return "The computer is still preparing music. Wait for preparation to finish, then sync again."}
            if code==409{return "The library changed during sync. Your downloads and edits are kept. Sync again to review the changes."}
            return details.count<180 && !details.contains("deserialize") ? details:"The computer couldn’t complete this request (\(code)). Try syncing again."
        }
    }
}
private func randomToken() throws -> String {
    var bytes = [UInt8](repeating: 0, count: 32)
    guard SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) == errSecSuccess else { throw CompanionError.message("Could not create the phone identity") }
    return bytes.map { String(format: "%02x", $0) }.joined()
}
private enum Credentials {
    static let service = "com.musicos.companion.pairing"
    static func load() -> Connection? {
        var item: CFTypeRef?
        let q: [String: Any] = [kSecClass as String:kSecClassGenericPassword,kSecAttrService as String:service,kSecReturnData as String:true]
        guard SecItemCopyMatching(q as CFDictionary, &item) == errSecSuccess, let data = item as? Data else { return nil }
        return try? JSONDecoder().decode(Connection.self, from: data)
    }
    static func save(_ value: Connection?) throws {
        let q: [String:Any] = [kSecClass as String:kSecClassGenericPassword,kSecAttrService as String:service]
        guard let value=value else{SecItemDelete(q as CFDictionary);return}
        let bytes=try JSONEncoder().encode(value)
        let updated=SecItemUpdate(q as CFDictionary,[kSecValueData as String:bytes] as CFDictionary)
        if updated==errSecItemNotFound {
            var attributes=q;attributes[kSecValueData as String]=bytes;attributes[kSecAttrAccessible as String]=kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
            guard SecItemAdd(attributes as CFDictionary,nil)==errSecSuccess else{throw CompanionError.message("Could not save the pairing in Keychain")}
        }else if updated != errSecSuccess{throw CompanionError.message("Could not update the pairing in Keychain")}
    }
}
private final class PinnedConnection: NSObject, URLSessionDelegate, URLSessionTaskDelegate {
    let fingerprint: String
    init(_ fingerprint: String) { self.fingerprint = fingerprint.lowercased() }
    func urlSession(_ session: URLSession, didReceive challenge: URLAuthenticationChallenge, completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void) {
        guard challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust,
              let trust = challenge.protectionSpace.serverTrust,
              let certificate = SecTrustGetCertificateAtIndex(trust, 0) else { completionHandler(.cancelAuthenticationChallenge, nil); return }
        let digest = SHA256.hash(data: SecCertificateCopyData(certificate) as Data).map { String(format:"%02x", $0) }.joined()
        guard digest == fingerprint else { completionHandler(.cancelAuthenticationChallenge, nil); return }
        completionHandler(.useCredential, URLCredential(trust: trust))
    }
    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) { completionHandler(nil) }
}
@MainActor private final class Companion {
    static let shared = Companion()
    let root: URL
    lazy var listening = ListeningRelay(root: root)
    let discovery=MacDiscovery();var discoveryStop:Task<Void,Never>?
    var connection = Credentials.load()
    var library: [[String:Any]] = []
    var collections: [[String:Any]] = []
    var portraits:[String:String]=[:]
    var preferences=MobilePreferences()
    var musicHapticsAvailable=false
    var infoRead=Set<Int64>()
    var backgroundRecovery:Task<Void,Never>?
    var libraryChanged:(()->Void)?
    var outbox: [[String:Any]] = []
    var conflicts: [[String:Any]] = []
    var status: [String:Any] = ["state":"idle", "completed":0, "total":0] {didSet{SyncLiveStatus.shared.update(status,enabled:preferences.liveSync)}}
    var queue: [Int64] = []; var queueIndex = 0
    var queueEntryIds:[Int64]=[];var entryCounter:Int64=Int64(Date().timeIntervalSince1970*1000);var repeatMode="none";var shuffled=false;var originalQueue:[(Int64,Int64)]=[]
    var artworkPalettes:[String:[String:Any]]=[:]
    var warmedAsset:(id:Int64,asset:AVURLAsset)?;var warmTask:Task<Void,Never>?
    let selectionFeedback=UISelectionFeedbackGenerator();let softFeedback=UIImpactFeedbackGenerator(style:.soft);var lastFeedback:TimeInterval=0
    var preparationMilliseconds=0.0
    var wantsPlayback=false;var preparing=false;var decodedJobs:[String:Task<Int32,Never>]=[:];var savedPosition=0.0;var lastSaved=0.0
    var itemObservation:NSKeyValueObservation?
    var cachedArtPath:String?;var cachedArtwork:MPMediaItemArtwork?
    var player = AVPlayer(); var observer: Any?; var notifications: [NSObjectProtocol] = []
    var emit: ((String)->Void)?
    var syncTask: Task<Void,Never>?; var prepareTask: Task<Void,Never>?
    private var syncSession: (base: URL, fingerprint: String, session: URLSession)?
    private var stoppingSync = false
    private var transferPlan: URL { root.appendingPathComponent("transfer-plan.json") }
    var preparingMix:Task<Void,Never>?,mixTask:Task<Void,Never>?
    var preparedMix:(deck:AVPlayer,index:Int,entry:Int64,source:Int64)?,outgoingMix:AVPlayer?
    var generation = 0; var resumeAfterInterruption = false; var playbackError: String?
    var currentID: Int64? { queue.indices.contains(queueIndex) ? queue[queueIndex] : nil }
    var current: [String:Any]? { library.first { number($0["id"]) == currentID } }
    func number(_ value: Any?) -> Int64 { (value as? NSNumber)?.int64Value ?? -1 }
    init() {
        root = FileManager.default.urls(for:.applicationSupportDirectory, in:.userDomainMask)[0].appendingPathComponent("musicOS", isDirectory:true)
        try? FileManager.default.createDirectory(at:root, withIntermediateDirectories:true)
        for name in ["audio", "artwork", "lyrics", "decoded", "partial"] { try? FileManager.default.createDirectory(at:root.appendingPathComponent(name), withIntermediateDirectories:true) }
        if let data = try? Data(contentsOf:root.appendingPathComponent("library.json")), let state = try? JSONSerialization.jsonObject(with:data) as? [String:Any] {
            preferences=MobilePreferences(state["preferences"] as? [String:Any] ?? [:])
            portraits=state["portraits"] as? [String:String] ?? [:];library = state["tracks"] as? [[String:Any]] ?? []; collections = state["collections"] as? [[String:Any]] ?? []
            outbox = state["outbox"] as? [[String:Any]] ?? []; conflicts = state["conflicts"] as? [[String:Any]] ?? []
            queue = (state["queue"] as? [NSNumber] ?? []).map(\.int64Value); queueIndex = state["queue_index"] as? Int ?? 0
            savedPosition=state["position"] as? Double ?? 0;repeatMode=state["repeat"] as? String ?? "none"
            queueEntryIds=(state["queue_entries"] as? [NSNumber] ?? []).map(\.int64Value)
            if queueEntryIds.count != queue.count{queueEntryIds=queue.map{_ in entryCounter+=1;return entryCounter}}
        }
        // iOS can relocate the data container during an update. Persist only library-relative assets.
        library=library.map{restorePaths($0)};collections=collections.map{restorePaths($0)};portraits=portraits.mapValues{restorePath($0)}
        configureBackgroundTransfers()
        attachPlaybackObserver()
        let center = MPRemoteCommandCenter.shared()
        center.playCommand.addTarget { [weak self] _ in Task { @MainActor in try? await self?.play() }; return .success }
        center.pauseCommand.addTarget { [weak self] _ in Task { @MainActor in self?.pause() }; return .success }
        center.togglePlayPauseCommand.addTarget { [weak self] _ in Task { @MainActor in try? await self?.toggle() }; return .success }
        center.nextTrackCommand.addTarget { [weak self] _ in Task { @MainActor in try? await self?.skip(1) }; return .success }
        center.previousTrackCommand.addTarget { [weak self] _ in Task { @MainActor in try? await self?.skip(-1) }; return .success }
        center.changePlaybackPositionCommand.addTarget { [weak self] event in
            guard let e = event as? MPChangePlaybackPositionCommandEvent else { return .commandFailed }
            Task { @MainActor in self?.seek(e.positionTime) }; return .success
        }
        notifications.append(NotificationCenter.default.addObserver(forName:.AVPlayerItemDidPlayToEndTime, object:nil, queue:.main) { [weak self] note in
            Task { @MainActor in guard note.object as? AVPlayerItem === self?.player.currentItem else { return }; try? await self?.skip(1, end:true) }
        })
        notifications.append(NotificationCenter.default.addObserver(forName:AVAudioSession.interruptionNotification, object:nil, queue:.main) { [weak self] note in
            let type = (note.userInfo?[AVAudioSessionInterruptionTypeKey] as? NSNumber)?.uintValue
            let options = (note.userInfo?[AVAudioSessionInterruptionOptionKey] as? NSNumber)?.uintValue ?? 0
            Task { @MainActor in
                guard let self = self else { return }
                if type == AVAudioSession.InterruptionType.began.rawValue { self.resumeAfterInterruption = self.player.rate > 0; self.pause() }
                else if self.resumeAfterInterruption && AVAudioSession.InterruptionOptions(rawValue:options).contains(.shouldResume) { self.resumeAfterInterruption = false; try? await self.play() }
            }
        })
        notifications.append(NotificationCenter.default.addObserver(forName:AVAudioSession.routeChangeNotification, object:nil, queue:.main) { [weak self] note in
            let reason = (note.userInfo?[AVAudioSessionRouteChangeReasonKey] as? NSNumber)?.uintValue
            Task { @MainActor in self?.cancelMix();if reason == AVAudioSession.RouteChangeReason.oldDeviceUnavailable.rawValue { self?.pause() }; self?.publish() }
        })
    }
    func configureBackgroundTransfers(){
        BackgroundTransfers.shared.approvedPin={[weak self] in self?.connection.map{(host:$0.host,fingerprint:$0.fingerprint)}}
        BackgroundTransfers.shared.ready={[weak self] in
            guard let self=self,self.backgroundRecovery==nil else{return}
            self.backgroundRecovery=Task {
                try? await Task.sleep(nanoseconds:800_000_000)
                if self.syncTask == nil && !self.stoppingSync { try? self.recoverBackgroundPlan() }
                self.backgroundRecovery=nil
            }
        }
        BackgroundTransfers.shared.progress={[weak self] done,total in
            guard let self=self,self.syncTask==nil,!self.stoppingSync,total>0 else{return}
            self.status = done == total
                ? ["state":"updating","message":"Adding verified files to your offline library."]
                : ["state":"downloading","completed":done,"total":total,"unit":"files","message":"Copying files in the background."]
        }
        try? recoverBackgroundPlan()
    }
    func backgroundAssets(_ manifest:[String:Any],connection:Connection)->[BackgroundAsset]{
        var values:[String:BackgroundAsset]=[:]
        func add(_ hash:String?,path:String,kind:String,size:Int64?=nil){guard let hash=hash,hash.count==64,hash.allSatisfy(\.isHexDigit)else{return};values[hash]=BackgroundAsset(hash:hash,path:path,kind:kind,size:size,fingerprint:connection.fingerprint)}
        for track in manifest["tracks"] as? [[String:Any]] ?? []{
            let hash=track["sha256"] as? String;add(hash,path:"tracks/\(number(track["id"]))",kind:"audio",size:number(track["size"]))
            if let hash=track["artwork_sha256"] as? String{add(hash,path:"artwork/\(hash)",kind:"artwork")}
            if let hash=track["lyrics_sha256"] as? String{add(hash,path:"assets/\(hash)",kind:"lyrics")}
        }
        for hash in (manifest["portraits"] as? [String:String] ?? [:]).values{add(hash,path:"artwork/\(hash)",kind:"artwork")}
        for row in manifest["collections"] as? [[String:Any]] ?? []{if let hash=row["artwork_sha256"] as? String{add(hash,path:"artwork/\(hash)",kind:"artwork")}}
        return values.values.sorted{$0.hash<$1.hash}
    }
    func recoverBackgroundPlan() throws {
        guard let connection=connection,let data=try? Data(contentsOf:transferPlan),let manifest=try? JSONSerialization.jsonObject(with:data) as? [String:Any],manifest["server_id"] as? String==connection.server else{return}
        var changed=false
        for var row in manifest["tracks"] as? [[String:Any]] ?? []{
            guard let hash=row["sha256"] as? String else{continue};let path=root.appendingPathComponent("audio/\(hash)")
            guard FileManager.default.fileExists(atPath:path.path)else{continue}
            let id=number(row["id"]);row["path"]=path.path;row["server_id"]=connection.server
            for (field,kind) in [("artwork","artwork"),("lyrics","lyrics")]{if let hash=row[field+"_sha256"] as? String{let file=root.appendingPathComponent("\(kind)/\(hash)");if FileManager.default.fileExists(atPath:file.path){row[field+"_path"]=file.path}}}
            if let index=library.firstIndex(where:{number($0["id"])==id}){
                let old=library[index];if outbox.contains(where:{$0["kind"] as? String=="favorite"&&number($0["track_id"])==id}){row["favorited"]=old["favorited"]}
                row["play_count"]=old["play_count"] ?? 0
                if old["lyrics_edited"] as? Bool==true{row["lyrics_path"]=old["lyrics_path"];row["lyrics_edited"]=true}
                if ["path","artwork_path","lyrics_path","sha256"].contains(where:{String(describing:old[$0]) != String(describing:row[$0])}){library[index]=row;changed=true}
            }else{library.append(row);changed=true}
        }
        for (name,hash) in manifest["portraits"] as? [String:String] ?? [:]{let path=root.appendingPathComponent("artwork/\(hash)").path;if FileManager.default.fileExists(atPath:path),portraits[name] != path{portraits[name]=path;changed=true}}
        for var row in manifest["collections"] as? [[String:Any]] ?? []{
            let id=row["id"] as? String ?? "";if outbox.contains(where:{$0["collection_id"] as? String==id}){continue}
            let ids=(row["full_track_ids"] as? [NSNumber] ?? row["track_ids"] as? [NSNumber] ?? []).map(\.int64Value);row["track_ids"]=ids;row["complete"]=ids.allSatisfy{id in library.contains{number($0["id"])==id}}
            if let hash=row["artwork_sha256"] as? String{let file=root.appendingPathComponent("artwork/\(hash)");if FileManager.default.fileExists(atPath:file.path){row["artwork_path"]=file.path}}
            let old=collections.first{$0["id"] as? String==id};if old==nil || json(old!) != json(row){collections.removeAll{$0["id"] as? String==id};collections.append(row);changed=true}
        }
        if changed{try persist();if UIApplication.shared.applicationState != .background{libraryChanged?()}}
        let assets = backgroundAssets(manifest,connection:connection)
        if !assets.isEmpty, assets.allSatisfy({ FileManager.default.fileExists(atPath:BackgroundTransfers.shared.destination($0).path) }) {
            status = ["state":"complete","message":outbox.isEmpty ? "Your downloads are ready." : "Your downloads are ready. Sync again to send changes to your Mac."]
        }
    }
    func warmUpcomingAudio(){
        warmTask?.cancel()
        guard queue.indices.contains(queueIndex+1),let next=library.first(where:{number($0["id"])==queue[queueIndex+1]})else{warmedAsset=nil;return}
        let id=number(next["id"])
        if warmedAsset?.id==id{return}
        warmedAsset=nil
        warmTask=Task{do{let url=try await preparedURL(next);try Task.checkCancellation();let asset=AVURLAsset(url:url);_ = try await asset.load(.duration);try Task.checkCancellation();warmedAsset=(id,asset)}catch{}}
    }
    func feedback(_ kind:String){
        guard preferences.interactionFeedback else{return}
        let now=ProcessInfo.processInfo.systemUptime;guard now-lastFeedback>0.055 else{return};lastFeedback=now
        if kind=="selection"{selectionFeedback.selectionChanged();selectionFeedback.prepare()}
        else{softFeedback.impactOccurred(intensity:kind=="drop" ? 0.8:kind=="pickup" ? 0.6:0.4);softFeedback.prepare()}
    }
    func attachPlaybackObserver(){
        observer=player.addPeriodicTimeObserver(forInterval:CMTime(seconds:0.2,preferredTimescale:600),queue:.main){[weak self]_ in Task{@MainActor in
            guard let self=self else{return};self.publish();self.considerMix()
            if self.player.rate>0,Date().timeIntervalSince1970-self.lastSaved>10{self.lastSaved=Date().timeIntervalSince1970;try? self.persist()}
        }}
    }
    func cancelMix(){
        preparingMix?.cancel();preparingMix=nil;mixTask?.cancel();mixTask=nil
        if let deck=preparedMix?.deck,deck !== player{deck.pause()};preparedMix=nil
        if let deck=outgoingMix,deck !== player{deck.pause()};outgoingMix=nil;player.volume=1
    }
    func preparedURL(_ track:[String:Any]) async throws -> URL {
        guard let path=track["path"] as? String else{throw CompanionError.message("Download this song first")}
        let native=["audio/mpeg","audio/mp4","audio/aac","audio/wav","audio/x-wav","audio/aiff","audio/x-m4a"]
        if native.contains(track["mime"] as? String ?? ""){return URL(fileURLWithPath:path)}
        let destination=root.appendingPathComponent("decoded/\(track["sha256"] as? String ?? UUID().uuidString).wav"),key=track["sha256"] as? String ?? path
        if FileManager.default.fileExists(atPath:destination.path){return destination}
        let job=decodedJobs[key] ?? Task.detached(priority:.userInitiated){path.withCString{s in destination.path.withCString{d in prepareAudio(s,d)}}}
        decodedJobs[key]=job;let code=await job.value;decodedJobs.removeValue(forKey:key);try Task.checkCancellation()
        guard code==0 else{throw CompanionError.message("This audio file could not be decoded")};return destination
    }
    func considerMix(){
        guard preferences.automix,player.rate>0,repeatMode != "one",mixTask==nil,let track=current,queue.indices.contains(queueIndex),queueEntryIds.indices.contains(queueIndex)else{return}
        let duration=(track["duration_ms"] as? Double ?? 0)/1000,remaining=duration-player.currentTime().seconds
        guard duration>6,remaining>0.12,remaining<=8 else{return}
        let index=queueIndex+1<queue.count ? queueIndex+1:repeatMode=="all" ? 0:-1
        guard queue.indices.contains(index),index != queueIndex,let next=library.first(where:{number($0["id"])==queue[index]})else{return}
        // Contiguous album sequences retain their natural endings.
        if track["album_id"] as? String==next["album_id"] as? String,number(next["track_number"])==number(track["track_number"])+1{return}
        let source=queueEntryIds[queueIndex],entry=queueEntryIds[index]
        if preparedMix==nil,preparingMix==nil{
            preparingMix=Task{[weak self] in
                guard let self=self else{return};defer{self.preparingMix=nil}
                do{let url=try await self.preparedURL(next);try Task.checkCancellation()
                    guard self.preferences.automix,self.queueEntryIds.indices.contains(self.queueIndex),self.queueEntryIds[self.queueIndex]==source else{return}
                    let deck=AVPlayer(playerItem:AVPlayerItem(url:url));deck.volume=0
                    for _ in 0..<50{if deck.currentItem?.status != .unknown{break};try await Task.sleep(nanoseconds:20_000_000)}
                    guard deck.currentItem?.status == .readyToPlay else{return}
                    self.preparedMix=(deck,index,entry,source);self.considerMix()
                }catch{} // A late/failed preparation retains the ordinary ending and skip path.
            }
        }
        guard remaining<=2,let ready=preparedMix,ready.source==source,ready.entry==entry else{return}
        let from=player,start=from.currentTime().seconds,length=min(2,remaining),to=ready.deck
        outgoingMix=from;to.play()
        mixTask=Task{[weak self] in
            guard let self=self else{return};var committed=false
            do{while !Task.isCancelled{
                let phase=min(1,max(0,(from.currentTime().seconds-start)/length))
                from.volume=Float(cos(phase * .pi/2));to.volume=Float(sin(phase * .pi/2))
                if phase>=0.5 && !committed{
                    guard self.queueEntryIds.indices.contains(ready.index),self.queueEntryIds[ready.index]==entry else{self.cancelMix();return}
                    committed=true;if let observer=self.observer{from.removeTimeObserver(observer)};self.observer=nil
                    self.player=to;self.queueIndex=ready.index;self.savedPosition=0;self.generation+=1;self.preparedMix=nil;self.attachPlaybackObserver();self.publish();try? self.persist();Task{await self.readRecordingInfo(next)}
                }
                if phase>=1{break};try await Task.sleep(nanoseconds:20_000_000)
            }}catch{}
            if !Task.isCancelled{from.pause();to.volume=1;self.outgoingMix=nil;self.mixTask=nil;self.publish()}
        }
    }
    func restorePath(_ path:String)->String {
        if let range=path.range(of:"/Library/Application Support/musicOS/"){return root.appendingPathComponent(String(path[range.upperBound...])).path}
        if !path.hasPrefix("/") && !path.split(separator:"/").contains(".."){return root.appendingPathComponent(path).path}
        return path
    }
    func restorePaths(_ row:[String:Any])->[String:Any]{var result=row;for key in ["path","artwork_path","lyrics_path"]{if let path=row[key] as? String{result[key]=restorePath(path)}};return result}
    func storePath(_ path:String)->String {let prefix=root.path+"/";return path.hasPrefix(prefix) ? String(path.dropFirst(prefix.count)):path}
    func storePaths(_ row:[String:Any])->[String:Any]{var result=row;for key in ["path","artwork_path","lyrics_path"]{if let path=row[key] as? String{result[key]=storePath(path)}};return result}
    func sceneUpdated(){publish();try? persist()}
    func persist() throws {
        let value: [String:Any] = ["version":1,"preferences":preferences.json,"portraits":portraits.mapValues{storePath($0)},"tracks":library.map{storePaths($0)},"collections":collections.map{storePaths($0)},"outbox":outbox,"conflicts":conflicts,"queue":queue,"queue_index":queueIndex,"queue_entries":queueEntryIds,"position":player.currentItem==nil ? savedPosition:max(0,player.currentTime().seconds.isFinite ? player.currentTime().seconds:0),"repeat":repeatMode]
        try JSONSerialization.data(withJSONObject:value, options:[.sortedKeys]).write(to:root.appendingPathComponent("library.json"), options:.atomic)
    }
    func json(_ value: Any) -> String { String(data:(try? JSONSerialization.data(withJSONObject:value, options:[.sortedKeys])) ?? Data("{}".utf8), encoding:.utf8)! }
    func playback() -> [String:Any] {
        let t = current.map(trackPresentation)
        let time = player.currentTime().seconds
        return ["track":t as Any? ?? NSNull(), "is_playing":player.rate > 0,"position":player.currentItem==nil ? savedPosition:(time.isFinite ? time : 0),"duration":(current?["duration_ms"] as? Double ?? 0)/1000,"queue":queue,"queue_index":queue.isEmpty ? NSNull() : queueIndex as Any,"queue_entry_ids":queueEntryIds,"volume":1,"shuffle":shuffled,"repeat":repeatMode,"preparing":preparing,"text_scale":UIFont.preferredFont(forTextStyle:.body).pointSize/17,"audio_levels":[],"error":playbackError as Any? ?? NSNull()]
    }
    func trackPresentation(_ t: [String:Any]) -> [String:Any] {
        var row=t; row["duration"] = (t["duration_ms"] as? Double ?? 0)/1000
        row["path"] = t["path"] ?? ""; row["cover_art_path"] = t["artwork_path"] ?? NSNull()
        row["available"]=true; row["play_count"] = t["play_count"] ?? 0; row["artist_display"] = t["artist"]
        return row
    }
    func snapshot() -> [String:Any] { ["live_sync_supported":SyncLiveStatus.shared.supported,"preferences":preferences.json,"music_haptics_active":musicHapticsActive,"music_haptics_available":musicHapticsAvailable,"portraits":portraits,"nearby_macs":discovery.results,"tracks":library.map(trackPresentation),"collections":collections,"playback":playback(),"sync":status,"paired":connection != nil,"pending_changes":outbox.count,"conflicts":conflicts,"text_scale":UIFont.preferredFont(forTextStyle:.body).pointSize/17,"storage_bytes":library.reduce(Int64(0)) { $0 + number($1["size"]) }] }
    var musicHapticsActive:Bool {if #available(iOS 18.0,*){return MAMusicHapticsManager.shared.isActive};return false}
    func readRecordingInfo(_ track:[String:Any]) async {
        let id=number(track["id"]);guard !infoRead.contains(id),let path=track["path"] as? String else{return};infoRead.insert(id)
        let values=await Task.detached(priority:.utility){()->[String:String] in
            var buffer=[CChar](repeating:0,count:512)
            let count=path.withCString{source in buffer.withUnsafeMutableBufferPointer{readAudioInfo(source,$0.baseAddress!,$0.count)}}
            guard count>0,let data=String(cString:buffer).data(using:.utf8)else{return [:]}
            return (try? JSONSerialization.jsonObject(with:data)) as? [String:String] ?? [:]
        }.value
        if let index=library.firstIndex(where:{number($0["id"])==id}){for(key,value)in values{library[index][key]=value};try? persist()}
        guard currentID==id else{return};musicHapticsAvailable=false
        if #available(iOS 18.0,*),preferences.musicHaptics,MAMusicHapticsManager.shared.isActive,let code=values["isrc"],code.count>=12{musicHapticsAvailable=await MAMusicHapticsManager.shared.isHapticTrackAvailable(forMediaMatching:code)}
        publish()
    }
    func writePlaybackDiagnostics(){
        func describe(_ error:Error?)->[String:Any]{guard let error=error as NSError? else{return [:]};let underlying=error.userInfo[NSUnderlyingErrorKey] as? NSError;return ["domain":error.domain,"code":error.code,"message":error.localizedDescription,"underlying_domain":underlying?.domain ?? "","underlying_code":underlying?.code ?? 0]}
        let file=player.currentItem?.asset as? AVURLAsset
        let session=AVAudioSession.sharedInstance()
        let info:[String:Any]=["version":3,"preparation_ms":preparationMilliseconds,"category":session.category.rawValue,"options":session.categoryOptions.rawValue,"route":session.currentRoute.outputs.map{$0.portType.rawValue},"rate":player.rate,"device_volume":session.outputVolume,"volume":player.volume,"muted":player.isMuted,"time_control":player.timeControlStatus.rawValue,"waiting":player.reasonForWaitingToPlay?.rawValue ?? "","item_status":player.currentItem?.status.rawValue ?? -1,"player_status":player.status.rawValue,"item_error":describe(player.currentItem?.error),"player_error":describe(player.error),"has_file":file.map{FileManager.default.fileExists(atPath:$0.url.path)} ?? false,"track_id":currentID ?? -1,"position":player.currentTime().seconds.isFinite ? player.currentTime().seconds:0,"wants_playback":wantsPlayback]
        try? JSONSerialization.data(withJSONObject:info,options:[.sortedKeys,.prettyPrinted]).write(to:root.appendingPathComponent("playback-diagnostics.json"),options:.atomic)
    }
    func publish() {
        if let t=current {
            var info: [String:Any] = [MPMediaItemPropertyTitle:t["title"] ?? "",MPMediaItemPropertyArtist:t["artist"] ?? "",MPMediaItemPropertyAlbumTitle:t["album"] ?? "",MPMediaItemPropertyPlaybackDuration:(t["duration_ms"] as? Double ?? 0)/1000,MPNowPlayingInfoPropertyElapsedPlaybackTime:max(0,player.currentTime().seconds.isFinite ? player.currentTime().seconds : 0),MPNowPlayingInfoPropertyPlaybackRate:player.rate]
            if #available(iOS 18.0,*),preferences.musicHaptics,let isrc=t["isrc"] as? String{info[MPNowPlayingInfoPropertyInternationalStandardRecordingCode]=isrc}
            let artPath=t["artwork_path"] as? String
            if cachedArtPath != artPath{cachedArtPath=artPath;cachedArtwork=artPath.flatMap{UIImage(contentsOfFile:$0)}.map{image in MPMediaItemArtwork(boundsSize:image.size){_ in image}}}
            if let art=cachedArtwork{info[MPMediaItemPropertyArtwork]=art}
            MPNowPlayingInfoCenter.default().nowPlayingInfo=info
        } else {MPNowPlayingInfoCenter.default().nowPlayingInfo=nil}
        observeListening()
        if UIApplication.shared.applicationState != .background { emit?(json(playback())) }
    }
    func loadCurrent() async throws {
        guard let t=current, t["path"] as? String != nil else { throw CompanionError.message("Choose a downloaded song first") }
        cancelMix();generation += 1; let request=generation
        preparing=true;publish();let began=ProcessInfo.processInfo.systemUptime;defer{preparing=false;preparationMilliseconds=(ProcessInfo.processInfo.systemUptime-began)*1000}
        let url=try await preparedURL(t)
        guard request==generation else { return }
        let asset=warmedAsset?.id==number(t["id"]) ? warmedAsset!.asset:AVURLAsset(url:url);if warmedAsset?.id==number(t["id"]){warmedAsset=nil};let item=AVPlayerItem(asset:asset);player.replaceCurrentItem(with:item)
        itemObservation=item.observe(\.status,options:[.initial,.new]){[weak self] item,_ in Task{@MainActor in
            guard let self=self,item === self.player.currentItem else{return}
            if item.status == .failed{self.playbackError="This downloaded file couldn’t start playing. Its copy is kept; try another song while musicOS checks the decoder.";self.wantsPlayback=false;self.writePlaybackDiagnostics();self.publish()}
        }}
        Task{await readRecordingInfo(t)};if savedPosition>0{await player.seek(to:CMTime(seconds:savedPosition,preferredTimescale:600),toleranceBefore:.zero,toleranceAfter:.zero)};playbackError=nil;publish()
    }
    func play() async throws {
        wantsPlayback=true
        do {
            // Playback already supports AirPlay. allowAirPlay is only valid for playAndRecord.
            try AVAudioSession.sharedInstance().setCategory(.playback,mode:.default,options:[])
            try AVAudioSession.sharedInstance().setActive(true)
            if player.currentItem==nil {try await loadCurrent()}
            guard wantsPlayback else{return}
            if player.currentItem?.status == .failed{writePlaybackDiagnostics();throw CompanionError.message("This audio format couldn’t start on the iPhone. The downloaded copy is kept.")}
            // Local verified files do not need the former three-second status poll.
            // AVPlayer owns readiness; item observation still reports decoder errors.
            player.automaticallyWaitsToMinimizeStalling=false;player.playImmediately(atRate:1);playbackError=nil;publish();writePlaybackDiagnostics();warmUpcomingAudio()
            let identity=currentID;Task{try? await Task.sleep(nanoseconds:1_000_000_000);if currentID==identity{writePlaybackDiagnostics()}}
        } catch {
            wantsPlayback=false;playbackError=(error as NSError).domain==NSOSStatusErrorDomain ? "The iPhone couldn’t start its audio output. Check the selected output and try Play again.":error.localizedDescription
            writePlaybackDiagnostics();publish();throw error
        }
    }
    func pause() {cancelMix();wantsPlayback=false;generation+=1;player.pause();try? persist();publish()}
    func toggle() async throws { if player.rate>0||wantsPlayback { pause() } else { try await play() } }
    func seek(_ seconds: Double) { cancelMix();player.seek(to:CMTime(seconds:max(0,seconds),preferredTimescale:600),toleranceBefore:.zero,toleranceAfter:.zero) { [weak self] _ in Task { @MainActor in self?.publish() } } }
    func skip(_ by:Int,end:Bool=false) async throws {
        cancelMix()
        guard !queue.isEmpty else { return }
        if end { listening.resetOccurrence() }
        if end&&repeatMode=="one"{seek(0);try await play();return}
        if by<0 && player.currentTime().seconds>3 { seek(0); return }
        let candidate=queueIndex+by;let next=(candidate>=queue.count&&repeatMode=="all") ? 0:candidate
        guard queue.indices.contains(next) else { if end { pause();seek(0) };return }
        queueIndex=next;savedPosition=0;player.pause();player.replaceCurrentItem(with:nil);try persist();try await play()
    }
    private func observeListening() {
        let sample: ListeningSample?
        if let id = currentID, let row = current, queueEntryIds.indices.contains(queueIndex) {
            let position = player.currentTime().seconds
            sample = ListeningSample(entry: queueEntryIds[queueIndex], trackID: id,
                position: position.isFinite ? position : savedPosition,
                duration: Double(number(row["duration_ms"])) / 1000,
                playing: player.rate > 0 && player.timeControlStatus == .playing,
                uptime: ProcessInfo.processInfo.systemUptime, date: Date())
        } else { sample = nil }
        let server = connection?.server
        listening.update(server: server, lastfm: preferences.lastfmListening,
                         discord: preferences.discordPresence, sample: sample) { [weak self] body in
            guard let self = self, self.connection?.server == server else { throw ListeningConnectionIssue.unpaired }
            return try await self.listeningRequest(body)
        }
    }

    private func listeningRequest(_ body: [String: Any]? = nil) async throws -> [String: Any] {
        guard connection != nil else { throw ListeningConnectionIssue.unpaired }
        do {
            return try await remote("listening", body: body) as? [String: Any] ?? [:]
        } catch CompanionError.server(404, _) {
            throw ListeningConnectionIssue.updateMac
        } catch CompanionError.server(403, _) {
            throw ListeningConnectionIssue.approval
        } catch CompanionError.server(401, _) {
            throw ListeningConnectionIssue.unpaired
        } catch {
            if let error = error as? URLError,
               [.cannotFindHost, .cannotConnectToHost, .networkConnectionLost, .timedOut].contains(error.code) {
                startDiscovery()
            }
            throw error
        }
    }

    private func startDiscovery() {
        discovery.finish = { [weak self] mac in
            guard let self = self, var connection = self.connection, mac["id"] == connection.server,
                  mac["fingerprint"] == connection.fingerprint,
                  let host = mac["host"], let port = Int(mac["port"] ?? ""),
                  host != connection.host || port != connection.port else { return }
            connection.host = host
            connection.port = port
            self.connection = connection
            try? Credentials.save(connection)
            self.observeListening()
            self.listening.tryAgain()
        }
        discovery.start()
        discoveryStop?.cancel()
        discoveryStop = Task {
            try? await Task.sleep(nanoseconds:5_000_000_000)
            if !Task.isCancelled { discovery.stop() }
        }
    }

    func request(_ payload:[String:Any]) async throws -> Any {
        switch payload["action"] as? String ?? "snapshot" {
        case "artwork_palette":
            guard let path=payload["path"] as? String,library.contains(where:{$0["artwork_path"] as? String==path}) || portraits.values.contains(path)else{throw CompanionError.message("This cover is not in your downloaded library")}
            if let value=artworkPalettes[path]{return value}
            let colors:[String]=await Task.detached(priority:.utility){var bytes=[CChar](repeating:0,count:512);let count=path.withCString{readArtworkColors($0,&bytes,bytes.count)};guard count>0,let data=String(cString:bytes).data(using:.utf8),let values=try? JSONSerialization.jsonObject(with:data) as? [String] else{return []};return values}.value
            guard colors.count==3 else{throw CompanionError.message("This cover’s colors are unavailable")}
            let value:[String:Any]=["background":colors[1],"blob":colors[2],"accent":colors[0],"text":"#12202a","swatches":colors];if artworkPalettes.count>=64{artworkPalettes.removeAll()};artworkPalettes[path]=value;return value
        case "settings":
            guard let key=payload["key"] as? String,let value=payload["value"] else{throw CompanionError.message("Choose a setting")}
            try preferences.set(key,value)
            if key=="lastfmListening" && !preferences.lastfmListening { listening.stopCollecting() }
            if !preferences.automix{cancelMix()};SyncLiveStatus.shared.update(status,enabled:preferences.liveSync);if preferences.interactionFeedback{UISelectionFeedbackGenerator().selectionChanged()};try persist();publish()
            if key=="discordPresence" { listening.tryAgain() }
            return snapshot()
        case "haptic":feedback(payload["kind"] as? String ?? "selection");return ["ok":true]
        case "snapshot": return snapshot()
        case "listening_status":
            observeListening()
            if payload["refresh"] as? Bool == true {
                do {
                    let result = try await listeningRequest()
                    listening.updateStatus(result)
                    if result["allowed"] as? Bool == true { listening.tryAgain() }
                } catch { listening.failed(error) }
            }
            return listening.status
        case "sync_status":
            return [
                "paired":connection != nil, "nearby_macs":discovery.results, "sync":status,
                "offline_songs":library.count, "storage_bytes":library.reduce(Int64(0)) { $0 + number($1["size"]) },
                "pending_changes":outbox.count, "conflicts":conflicts
            ] as [String:Any]
        case "playback": return playback()
        case "play":
            cancelMix()
            if let ids=payload["ids"] as? [NSNumber] { let values=ids.map(\.int64Value); guard !values.isEmpty,values.allSatisfy({ id in library.contains { number($0["id"])==id } }) else { throw CompanionError.message("Some songs have not downloaded yet") };generation+=1;player.pause();player.replaceCurrentItem(with:nil);queue=values;queueEntryIds=values.map{_ in entryCounter+=1;return entryCounter};originalQueue=[];shuffled=false;savedPosition=0;queueIndex=payload["index"] as? Int ?? 0;guard queue.indices.contains(queueIndex) else { throw CompanionError.message("Invalid queue position") };try persist() }
            try await play(); return playback()
        case "toggle": try await toggle(); return playback()
        case "pause": pause(); return playback()
        case "next": try await skip(1);return playback()
        case "previous": try await skip(-1);return playback()
        case "seek":
            cancelMix()
            let seconds=max(0,min(payload["position"] as? Double ?? 0,(current?["duration_ms"] as? Double ?? 0)/1000)),token=generation
            if player.currentItem != nil{_=await player.seek(to:CMTime(seconds:seconds,preferredTimescale:600),toleranceBefore:.zero,toleranceAfter:.zero)}else{savedPosition=seconds}
            if generation==token{publish()};return playback()
        case "queue_play":
            cancelMix()
            let index=payload["index"] as? Int ?? -1
            guard queue.indices.contains(index) else {throw CompanionError.message("Queue entry moved")}
            if index != queueIndex{generation+=1;player.pause();player.replaceCurrentItem(with:nil);queueIndex=index;savedPosition=0}
            try await play();try persist();return playback()
        case "queue_move":
            cancelMix()
            let from=payload["from"] as? Int ?? -1,to=payload["to"] as? Int ?? -1
            guard queue.indices.contains(from),queue.indices.contains(to) else { throw CompanionError.message("Invalid queue position") }
            let currentEntry=queueIndex;let id=queue.remove(at:from),entry=queueEntryIds.remove(at:from);queue.insert(id,at:to);queueEntryIds.insert(entry,at:to)
            if from==currentEntry { queueIndex=to } else if from<currentEntry && to>=currentEntry { queueIndex-=1 } else if from>currentEntry && to<=currentEntry { queueIndex+=1 }
            try persist();return playback()
        case "queue_add":
            cancelMix()
            let ids=(payload["ids"] as? [NSNumber] ?? []).map(\.int64Value)
            guard !ids.isEmpty,ids.allSatisfy({id in library.contains{number($0["id"])==id}})else{throw CompanionError.message("Download these songs first")}
            let at=payload["next"] as? Bool==true ? min(queueIndex+1,queue.count):queue.count
            queue.insert(contentsOf:ids,at:at);queueEntryIds.insert(contentsOf:ids.map{_ in entryCounter+=1;return entryCounter},at:at);try persist();publish();return playback()
        case "queue_clear_upcoming":
            cancelMix();let keep=min(queue.count,queueIndex+1);queue=Array(queue.prefix(keep));queueEntryIds=Array(queueEntryIds.prefix(keep));warmTask?.cancel();warmedAsset=nil;feedback("drop");try persist();publish();return playback()
        case "queue_remove":
            cancelMix()
            let index=payload["index"] as? Int ?? -1;guard queue.indices.contains(index)else{throw CompanionError.message("Queue entry moved")}
            let wasPlaying=player.rate>0;queue.remove(at:index);queueEntryIds.remove(at:index)
            if index<queueIndex{queueIndex-=1}else if index==queueIndex{generation+=1;player.pause();player.replaceCurrentItem(with:nil);savedPosition=0;queueIndex=min(queueIndex,max(0,queue.count-1));if wasPlaying && !queue.isEmpty{try await play()}}
            try persist();publish();return playback()
        case "shuffle":
            cancelMix()
            let entries=Array(zip(queueEntryIds,queue));if !shuffled{originalQueue=entries;let start=min(queueIndex+1,queue.count);let tail=Array(entries.dropFirst(start)).shuffled();queue=Array(queue.prefix(start))+tail.map{$0.1};queueEntryIds=Array(queueEntryIds.prefix(start))+tail.map{$0.0}}
            else{let start=min(queueIndex+1,queue.count),pending=Set(queueEntryIds.dropFirst(start));let tail=originalQueue.filter{pending.contains($0.0)}+entries.dropFirst(start).filter{entry in !originalQueue.contains{$0.0==entry.0}};queue=Array(queue.prefix(start))+tail.map{$0.1};queueEntryIds=Array(queueEntryIds.prefix(start))+tail.map{$0.0}}
            shuffled.toggle();try persist();publish();return playback()
        case "repeat":repeatMode=repeatMode=="none" ? "all":repeatMode=="all" ? "one":"none";try persist();publish();return playback()
        case "favorite":
            let id=number(payload["id"]),value=payload["value"] as? Bool ?? false
            guard let i=library.firstIndex(where:{number($0["id"])==id}) else { throw CompanionError.message("Song is unavailable") }
            let before=library[i]["favorited"] as? Bool ?? false; library[i]["favorited"]=value
            if preferences.interactionFeedback{UIImpactFeedbackGenerator(style:.soft).impactOccurred()}
            outbox.append(["id":UUID().uuidString,"kind":"favorite","track_id":id,"base":before,"value":value]);try persist();return snapshot()
        case "playlist":
            let id=payload["id"] as? String ?? "phone/\(UUID().uuidString)"
            let old=collections.first(where:{$0["id"] as? String==id})
            if let expected=payload["base_revision"] as? String,expected != (old?["revision"] as? String ?? ""){throw CompanionError.message("This playlist changed while you were editing. Your draft is kept. Reopen the playlist to use the latest version.")}
            let ids=payload["track_ids"] as? [NSNumber] ?? []
            guard ids.allSatisfy({ id in library.contains{number($0["id"])==id.int64Value} }) else { throw CompanionError.message("A playlist can only include downloaded songs") }
            let name=String((payload["name"] as? String ?? "Playlist").prefix(120)).trimmingCharacters(in:.whitespacesAndNewlines)
            guard !name.isEmpty else { throw CompanionError.message("Give the playlist a name") }
            let operationID=UUID().uuidString
            let row:[String:Any]=["id":id,"kind":"manual","name":name,"track_ids":ids,"revision":old?["revision"] ?? "","pending_change":operationID]
            collections.removeAll{$0["id"] as? String==id};collections.append(row)
            outbox.append(["id":operationID,"kind":"playlist","collection_id":id,"base_revision":old?["revision"] ?? "","depends_on":old?["pending_change"] ?? NSNull(),"name":name,"track_ids":ids]);try persist();return snapshot()
        case "lyrics":
            guard let t=library.first(where:{number($0["id"])==number(payload["id"])}),let path=t["lyrics_path"] as? String,let data=try? Data(contentsOf:URL(fileURLWithPath:path)) else { return ["document":NSNull(),"message":"No saved lyrics were copied for this song. Save lyrics on your Mac, then sync again.","state":"not_found","candidates":[],"revision":0,"track_id":number(payload["id"])] as [String:Any] }
            let doc=try JSONSerialization.jsonObject(with:data)
            return ["document":doc,"state":"ready","candidates":[],"revision":0,"track_id":number(payload["id"])] as [String:Any]
        case "pair": return try await pair(payload["link"] as? String ?? "")
        case "approval": return try await remote("pair/status")
        case "sync":
            guard syncTask == nil, !stoppingSync else { return status }
            status = ["state":"starting"]
            syncTask = Task {
                do { try await sync() }
                catch {
                    if Task.isCancelled || error is CancellationError {
                        status = ["state":"cancelled","message":"Downloaded songs are kept. Retry to continue."]
                    } else if let networkError = error as? URLError,
                              [.notConnectedToInternet, .timedOut, .networkConnectionLost, .cannotConnectToHost, .cannotFindHost].contains(networkError.code) {
                        status = ["state":"error","message":"Couldn’t reach your Mac. Check Wi-Fi and make sure sharing is on in Devices."]
                    } else {
                        status = ["state":"error","message":error.localizedDescription]
                    }
                }
                syncTask = nil
                if UIApplication.shared.applicationState != .background { libraryChanged?() }
            }
            return status
        case "cancel":
            await stopSync(keepResume:true)
            return status
        case "disconnect":
            await stopSync(keepResume:false)
            connection = nil
            observeListening()
            try Credentials.save(nil)
            status = ["state":"idle"]
            return snapshot()
        case "resolve":
            let id=payload["id"] as? String ?? "",phone=payload["phone"] as? Bool ?? false
            if let c=conflicts.first(where:{$0["id"] as? String==id}),phone,var op=c["operation"] as? [String:Any] { op["id"]=UUID().uuidString;op["force"]=true;outbox.append(op) }
            if !phone,let conflict=conflicts.first(where:{$0["id"] as? String==id}),let operation=conflict["operation"] as? [String:Any] {
                if operation["kind"] as? String=="favorite",let value=conflict["mac"] as? Bool,let index=library.firstIndex(where:{number($0["id"])==number(operation["track_id"])}){library[index]["favorited"]=value}
                if let row=conflict["mac"] as? [String:Any],let collection=row["id"] as? String{collections.removeAll{$0["id"] as? String==collection};collections.append(row)}
            }
            conflicts.removeAll{$0["id"] as? String==id};try persist();return snapshot()
        case "discover":
            startDiscovery()
            return discovery.results
        case "stop_discovery":discoveryStop?.cancel();discovery.stop();return [:] as [String:String]
        case "scan":
            return try await scanPairingCode()
        case "route":
            if let view=UIApplication.shared.connectedScenes.compactMap({$0 as? UIWindowScene}).flatMap(\.windows).first(where:{$0.isKeyWindow})?.rootViewController?.view {
                let route=AVRoutePickerView(frame:CGRect(x:view.bounds.midX,y:view.bounds.midY,width:44,height:44));view.addSubview(route);(route.subviews.first(where:{$0 is UIButton}) as? UIButton)?.sendActions(for:.touchUpInside);DispatchQueue.main.asyncAfter(deadline:.now()+1){route.removeFromSuperview()}
            };return [:] as [String:String]
        default: throw CompanionError.message("This operation is unavailable on iPhone")
        }
    }
    func remote(_ path:String,body:[String:Any]?=nil,range:String?=nil) async throws -> Any {
        let (data,_)=try await fetch(path,body:body,range:range)
        return try JSONSerialization.jsonObject(with:data)
    }
    func fetch(_ path:String,body:[String:Any]?=nil,range:String?=nil,etag:String?=nil) async throws -> (Data,HTTPURLResponse) {
        guard let c=connection else { throw CompanionError.message("Pair with your Mac first") }
        let shared = syncSession.flatMap { $0.base == c.base && $0.fingerprint == c.fingerprint ? $0.session : nil }
        let session = shared ?? makeSyncSession(fingerprint:c.fingerprint, listening:path=="listening")
        defer { if shared == nil { session.finishTasksAndInvalidate() } }
        var r=URLRequest(url:c.base.appendingPathComponent(path));r.setValue("Bearer \(c.client).\(c.token)",forHTTPHeaderField:"Authorization")
        if let body=body { r.httpMethod="POST";r.httpBody=try JSONSerialization.data(withJSONObject:body);r.setValue("application/json",forHTTPHeaderField:"Content-Type") }
        if let range=range {r.setValue(range,forHTTPHeaderField:"Range")};if let etag=etag {r.setValue("\"\(etag)\"",forHTTPHeaderField:"If-Match")}
        let (data,response)=try await session.data(for:r)
        guard let http=response as? HTTPURLResponse,(200...299).contains(http.statusCode) else {let code=(response as? HTTPURLResponse)?.statusCode ?? 0;throw CompanionError.server(code,String(data:data,encoding:.utf8) ?? "")}
        return (data,http)
    }
    func pair(_ link:String) async throws -> Any {
        guard let parts=URLComponents(string:link),parts.scheme=="musicos",parts.host=="pair" else { throw CompanionError.message("Paste a musicOS pairing link from Devices on the Mac") }
        let q=Dictionary((parts.queryItems ?? []).map{($0.name,$0.value ?? "")},uniquingKeysWith:{$1})
        guard let host=q["host"],let port=Int(q["port"] ?? ""),(1...65535).contains(port),let server=q["server"],let fingerprint=q["fingerprint"],fingerprint.count==64,fingerprint.allSatisfy(\.isHexDigit),let ticket=q["ticket"],!host.isEmpty else { throw CompanionError.message("The pairing link is incomplete") }
        // Pairing explicitly pins the Mac's certificate; never inherit browser cookies or trust redirects.
        let privateHost=host=="localhost"||host.hasSuffix(".local")||host.hasSuffix(".local.")||host.hasPrefix("10.")||host.hasPrefix("192.168.")||host.hasPrefix("169.254.")||host.hasPrefix("127.")||(host.split(separator:".").count==4&&host.hasPrefix("172.")&&(16...31).contains(Int(host.split(separator:".")[1]) ?? -1))
        guard privateHost else{throw CompanionError.message("Use the pairing code from a Mac on your local Wi-Fi network.")}
        let old=connection
        if library.contains(where:{$0["server_id"] as? String != server}) { throw CompanionError.message("This phone already has music from another Mac. Keep that pairing or remove its downloads before pairing a different library.") }
        connection=Connection(host:host,port:port,server:server,fingerprint:fingerprint,client:try randomToken(),token:try randomToken())
        do {
            let info=try await remote("info") as? [String:Any] ?? [:]
            let supported=(info["capabilities"] as? [String] ?? []).contains("collection-mutations-v1")
            connection?.syncChanges=supported
            var body:[String:Any]=["ticket":ticket,"client_id":connection!.client,"client_name":UIDevice.current.name,"client_token":connection!.token]
            if supported{body["capabilities"]=["collection-mutations-v1","lyric-assets-v1"]}
            let result:Any
            do{result=try await remote("pair",body:body)}
            catch CompanionError.server(422,let detail) where supported && detail.contains("capabilities"){
                body.removeValue(forKey:"capabilities");connection?.syncChanges=false;result=try await remote("pair",body:body)
            }
            try Credentials.save(connection)
            let approved = (result as? [String:Any])?["state"] as? String == "approved"
            status = approved ? ["state":"idle"] : ["state":"approval","message":"Approve this iPhone in Devices on your Mac, then retry sync."]
            return result
        }
        catch {connection=old;throw error}
    }
    func hashFile(_ file:URL) throws -> String {
        let handle=try FileHandle(forReadingFrom:file);defer{try? handle.close()};var hash=SHA256()
        while let chunk=try handle.read(upToCount:1<<20),!chunk.isEmpty {hash.update(data:chunk)}
        return hash.finalize().map{String(format:"%02x",$0)}.joined()
    }
    func download(_ path:String,hash:String,size:Int64?,kind:String) async throws -> URL {
        guard hash.count==64,hash.allSatisfy(\.isHexDigit) else {throw CompanionError.message("Invalid file fingerprint")}
        let file=root.appendingPathComponent("\(kind)/\(hash)")
        if preferences.backgroundSync,let connection=connection{return try await BackgroundTransfers.shared.value(BackgroundAsset(hash:hash,path:path,kind:kind,size:size,fingerprint:connection.fingerprint))}
        if FileManager.default.fileExists(atPath:file.path) {return file}
        let partial=root.appendingPathComponent("partial/\(hash)")
        if !FileManager.default.fileExists(atPath:partial.path){FileManager.default.createFile(atPath:partial.path,contents:nil)}
        let handle=try FileHandle(forWritingTo:partial);defer{try? handle.close()}
        var offset=Int64(try handle.seekToEnd())
        if let size=size,offset>size {try handle.truncate(atOffset:0);offset=0}
        repeat {
            try Task.checkCancellation()
            if let size=size,offset==size {break}
            let end=size.map{min($0-1,offset+2*1024*1024-1)} ?? offset+2*1024*1024-1
            let (data,response)=try await fetch(path,range:"bytes=\(offset)-\(end)",etag:hash)
            guard !data.isEmpty,data.count<=2*1024*1024,(offset==0 || response.statusCode==206) else { throw CompanionError.message("The Mac sent an invalid download range") }
            try handle.write(contentsOf:data);offset+=Int64(data.count)
            if size==nil {if let total=response.value(forHTTPHeaderField:"Content-Range")?.split(separator:"/").last,Int64(total)==offset {break};if response.statusCode==200{break}}
        }while true
        try handle.synchronize()
        let digest=try await Task.detached(priority:.utility) {try self.hashFileNonisolated(partial)}.value
        guard digest==hash else {try? FileManager.default.removeItem(at:partial);throw CompanionError.message("A downloaded file failed verification. Sync again to retry it.")}
        try FileManager.default.moveItem(at:partial,to:file);return file
    }
    nonisolated func hashFileNonisolated(_ file:URL) throws -> String {
        let handle=try FileHandle(forReadingFrom:file);defer{try? handle.close()};var hash=SHA256()
        while let data=try handle.read(upToCount:1<<20),!data.isEmpty {hash.update(data:data)}
        return hash.finalize().map{String(format:"%02x",$0)}.joined()
    }
    private func makeSyncSession(fingerprint: String, listening: Bool = false) -> URLSession {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = listening ? 5 : 30
        configuration.timeoutIntervalForResource = listening ? 8 : 120
        configuration.allowsCellularAccess = !listening
        return URLSession(configuration:configuration,delegate:PinnedConnection(fingerprint),delegateQueue:nil)
    }

    private func stopSync(keepResume: Bool) async {
        guard !stoppingSync else { return }
        stoppingSync = true
        defer { stoppingSync = false }
        status = ["state":"cancelling"]
        let job = syncTask
        job?.cancel()
        await BackgroundTransfers.shared.cancel(keepResume:keepResume)
        await job?.value
        try? FileManager.default.removeItem(at:transferPlan)
        status = ["state":"cancelled","message":"Downloaded songs are kept. Retry to continue."]
    }

    func sync() async throws {
        guard let mac = connection else { throw CompanionError.message("Pair with your Mac first") }
        let session = makeSyncSession(fingerprint:mac.fingerprint)
        syncSession = (mac.base,mac.fingerprint,session)
        defer { syncSession = nil; session.invalidateAndCancel() }
        status = ["state":"connecting"]
        let approval = try await remote("pair/status") as? [String:Any]
        guard approval?["state"] as? String == "approved" else {
            status = ["state":"approval","message":"Approve this iPhone in Devices on your Mac, then retry sync."]
            return
        }
        status = ["state":"preparing"]
        var attempts=0
        while let progress=try await remote("preparation") as? [String:Any],progress["preparing"] as? Bool==true {
            try Task.checkCancellation();attempts+=1;guard attempts<1500 else{throw CompanionError.message("The Mac is still preparing music. Sync again when it finishes.")}
            status=["state":"preparing","completed":progress["completed"] ?? 0,"total":progress["total"] ?? 0,"message":progress["current"] ?? "You can keep listening while your Mac prepares copies."]
            try await Task.sleep(nanoseconds:400_000_000)
        }
        let manifest=try await remote("manifest") as? [String:Any] ?? [:]
        guard manifest["version"] as? Int==1,manifest["server_id"] as? String==connection?.server,let tracks=manifest["tracks"] as? [[String:Any]] else {throw CompanionError.message("This Mac's library format is unavailable")}
        try Task.checkCancellation()
        let total = tracks.count
        var missingAudio = Set<String>()
        var required: Int64 = 0
        for track in tracks {
            guard let hash = track["sha256"] as? String, hash.count == 64, hash.allSatisfy(\.isHexDigit) else {
                throw CompanionError.message("The Mac sent an invalid song fingerprint. Refresh sharing on the Mac and retry.")
            }
            let path = root.appendingPathComponent("audio/\(hash)").path
            if !FileManager.default.fileExists(atPath:path), missingAudio.insert(hash).inserted {
                required += number(track["size"])
            }
        }
        let copyCount = missingAudio.count
        let values = try root.resourceValues(forKeys:[.volumeAvailableCapacityForImportantUsageKey])
        if let free = values.volumeAvailableCapacityForImportantUsage, required > free {
            throw CompanionError.message("Not enough space. Choose fewer songs on the Mac, then retry.")
        }
        status = copyCount == 0 ? ["state":"updating","message":"Your music is already on this iPhone. Checking artwork, lyrics and collections."]
            : ["state":"downloading","completed":0,"total":copyCount]
        if preferences.backgroundSync {
            try JSONSerialization.data(withJSONObject:manifest,options:[.sortedKeys]).write(to:transferPlan,options:.atomic)
            await BackgroundTransfers.shared.start(backgroundAssets(manifest,connection:mac),base:mac.base,authorization:"Bearer \(mac.client).\(mac.token)")
        }
        for var track in tracks {
            try Task.checkCancellation()
            let id = number(track["id"])
            guard let hash = track["sha256"] as? String else { continue }
            if missingAudio.contains(hash) {
                status = ["state":"downloading","completed":copyCount-missingAudio.count,"total":copyCount,"message":track["title"] ?? "Downloading music"]
            }
            let path = try await download("tracks/\(id)",hash:hash,size:number(track["size"]),kind:"audio")
            track["path"]=path.path;track["server_id"]=mac.server
            if let h=track["artwork_sha256"] as? String {track["artwork_path"]=try await download("artwork/\(h)",hash:h,size:nil,kind:"artwork").path}
            if let h=track["lyrics_sha256"] as? String {track["lyrics_path"]=try await download("assets/\(h)",hash:h,size:nil,kind:"lyrics").path}
            try Task.checkCancellation()
            if let index = library.firstIndex(where:{ number($0["id"]) == id }) {
                let old = library[index]
                if outbox.contains(where:{ $0["kind"] as? String == "favorite" && number($0["track_id"]) == id }) { track["favorited"] = old["favorited"] }
                if old["lyrics_edited"] as? Bool == true {
                    track["lyrics_path"] = old["lyrics_path"]
                    track["lyrics_edited"] = true
                }
                track["play_count"] = old["play_count"] ?? 0
                if !NSDictionary(dictionary:old).isEqual(to:track) {
                    library[index] = track
                    try persist()
                }
            } else {
                library.append(track)
                try persist()
            }
            if missingAudio.remove(hash) != nil {
                status = ["state":"downloading","completed":copyCount-missingAudio.count,"total":copyCount]
            }
        }
        status = ["state":"updating","message":"Finishing artwork, lyrics and collections."]
        for (name,hash) in manifest["portraits"] as? [String:String] ?? [:]{try Task.checkCancellation();portraits[name]=try await download("artwork/\(hash)",hash:hash,size:nil,kind:"artwork").path}
        // Never delete downloads merely because the Mac shared a smaller selection.
        for collection in manifest["collections"] as? [[String:Any]] ?? [] {
            let id=collection["id"] as? String ?? ""
            if outbox.contains(where:{$0["collection_id"] as? String==id}){continue}
            var row=collection
            if let hash=row["artwork_sha256"] as? String{row["artwork_path"]=try await download("artwork/\(hash)",hash:hash,size:nil,kind:"artwork").path}
            let ids=(row["full_track_ids"] as? [NSNumber] ?? row["track_ids"] as? [NSNumber] ?? []).map(\.int64Value)
            row["track_ids"]=ids
            row["complete"]=ids.allSatisfy{id in library.contains{number($0["id"])==id}}
            collections.removeAll{$0["id"] as? String==id};collections.append(row)
        }
        if let capabilities=manifest["capabilities"] as? [String],capabilities.contains("collection-mutations-v1"),connection?.syncChanges != false {
            status=["state":"uploading","message":"Syncing your changes…"]
            for operation in outbox {
                try Task.checkCancellation()
                let response=try await remote("changes",body:operation) as? [String:Any] ?? [:]
                guard let id=operation["id"] as? String else{continue}
                guard ["saved","conflict"].contains(response["state"] as? String ?? "") else{throw CompanionError.message("The Mac did not acknowledge your changes. They remain saved on this iPhone.")}
                if response["state"] as? String=="conflict" {conflicts.removeAll{$0["id"] as? String==id};conflicts.append(["id":id,"operation":operation,"mac":response["current"] ?? NSNull()])}
                else if let updated=response["collection"] as? [String:Any] {
                    let original=operation["collection_id"] as? String
                    if !outbox.contains(where:{$0["collection_id"] as? String==original&&$0["id"] as? String != id}){collections.removeAll{$0["id"] as? String==original};collections.append(updated)}
                }
                outbox.removeAll{$0["id"] as? String==id};try persist()
            }
        }
        try persist();_ = try? await remote("sync-complete",body:["revision":manifest["revision"] ?? "","track_count":total,"bytes":manifest["bytes_total"] ?? 0])
        try Task.checkCancellation()
        try? FileManager.default.removeItem(at:transferPlan)
        let message: String
        if !outbox.isEmpty && connection?.syncChanges == false {
            message = "Update musicOS on the Mac and pair again to sync favorites and playlists."
        } else if !conflicts.isEmpty {
            message = "Review sync conflicts below."
        } else if total == 0 {
            message = "No songs are shared. Choose music in Devices on your Mac."
        } else if copyCount == 0 {
            message = "Library checked. No new songs to copy."
        } else {
            message = "\(copyCount) \(copyCount == 1 ? "song" : "songs") copied."
        }
        status = ["state":"complete","completed":copyCount,"total":copyCount,"message":message]
    }
}
class CompanionPlugin: Plugin {
    override func load(webview:WKWebView){
        #if DEBUG
        if ProcessInfo.processInfo.environment["MUSICOS_COMPANION_SELF_TEST"]=="1"{Task{@MainActor in await Companion.shared.runNativeChecks()}}
        #endif
        webview.scrollView.pinchGestureRecognizer?.isEnabled=false
        webview.scrollView.minimumZoomScale=1;webview.scrollView.maximumZoomScale=1
        webview.isOpaque=true
        webview.backgroundColor=UIColor{trait in trait.userInterfaceStyle == .dark ? UIColor(red:0.06,green:0.15,blue:0.18,alpha:1):UIColor(red:0.65,green:0.82,blue:0.85,alpha:1)}
        NotificationCenter.default.addObserver(forName:UIApplication.didEnterBackgroundNotification,object:nil,queue:.main){_ in Task{@MainActor in Companion.shared.sceneUpdated()}}
        NotificationCenter.default.addObserver(forName:UIApplication.didBecomeActiveNotification,object:nil,queue:.main){_ in Task{@MainActor in Companion.shared.sceneUpdated()}}
        NotificationCenter.default.addObserver(forName:UIContentSizeCategory.didChangeNotification,object:nil,queue:.main){_ in Task{@MainActor in Companion.shared.sceneUpdated()}}
    }
    @objc public func request(_ invoke: Invoke) throws {
        let data=Data(invoke.getRawArgs().utf8)
        guard let args=try JSONSerialization.jsonObject(with:data) as? [String:Any] else {invoke.reject("Invalid request");return}
        Task { @MainActor in
            let service=Companion.shared
            service.emit={ [weak self] json in self?.trigger("playback",data:["json":json]) };service.libraryChanged={ [weak self] in self?.trigger("library",data:[:]) }
            do {invoke.resolve(service.json(try await service.request(args)))}catch{invoke.reject(error.localizedDescription)}
        }
    }
}
@_cdecl("init_plugin_companion")
func initPlugin() -> Plugin { CompanionPlugin() }

#if DEBUG
extension Companion {
    func runNativeChecks() async {
        var checks:[[String:Any]]=[]
        func check(_ name:String,_ passed:Bool){checks.append(["name":name,"passed":passed])}
        // The harness refuses ordinary libraries, including the user's phone.
        guard !library.isEmpty,library.allSatisfy({($0["path"] as? String ?? "").hasPrefix(root.appendingPathComponent("test-fixtures").path+"/")})else{return}
        do {
            check("startup stays paused",player.rate==0)
            if let baseText=ProcessInfo.processInfo.environment["MUSICOS_TEST_SYNC_BASE"],let base=URL(string:baseText),let pin=ProcessInfo.processInfo.environment["MUSICOS_TEST_SYNC_PIN"],let hash=ProcessInfo.processInfo.environment["MUSICOS_TEST_SYNC_HASH"]{
                let service=BackgroundTransfers.shared;service.approvedPin={(host:"127.0.0.1",fingerprint:pin)}
                let asset=BackgroundAsset(hash:hash,path:"fixture",kind:"artwork",size:nil,fingerprint:pin)
                try? FileManager.default.removeItem(at:service.destination(asset))
                await service.start([asset],base:base,authorization:"Bearer isolated-test")
                let file=try await service.value(asset);check("background URLSession pins and verifies a downloaded asset",try hashFileNonisolated(file)==hash)
                await service.cancel(keepResume:false)
            }
            if let art=library.first?["artwork_path"] as? String{let palette=try await request(["action":"artwork_palette","path":art]) as? [String:Any];check("downloaded cover colors use the native SDR extractor",(palette?["swatches"] as? [String])?.count==3)}
            let relative="test-fixtures/original.flac";check("asset paths survive container relocation",restorePath("/old/Library/Application Support/musicOS/"+relative)==root.appendingPathComponent(relative).path && storePath(root.appendingPathComponent(relative).path)==relative)
            let ids=library.map{number($0["id"])}
            for id in ids {
                _=try await request(["action":"play","ids":[id]])
                try await Task.sleep(nanoseconds:700_000_000)
                check("playback uses a valid device audio session \(id)",AVAudioSession.sharedInstance().category == .playback && !AVAudioSession.sharedInstance().categoryOptions.contains(.allowAirPlay))
                check("offline audio advances \(id)",player.rate>0&&player.currentTime().seconds>0.1)
                seek(1);try await Task.sleep(nanoseconds:160_000_000);pause()
                check("seek acknowledged \(id)",abs(player.currentTime().seconds-1)<0.3)
                check("pause holds \(id)",player.rate==0)
            }
            let first=ids[0],last=ids.last!
            _=try await request(["action":"play","ids":[first,last,first]])
            let identities=queueEntryIds
            _=try await request(["action":"queue_move","from":2,"to":1])
            check("duplicate queue identities survive reordering",queue==[first,first,last]&&queueEntryIds==[identities[0],identities[2],identities[1]]&&queueIndex==0)
            _=try await request(["action":"shuffle"]);_=try await request(["action":"shuffle"])
            check("shuffle keeps current entry",queueEntryIds[queueIndex]==identities[0])
            _=try await request(["action":"queue_clear_upcoming"])
            check("clearing upcoming preserves current song and entry",queue==[first]&&queueEntryIds==[identities[0]]&&queueIndex==0)
            preferences.interactionFeedback=true;lastFeedback=0;feedback("selection");let tick=lastFeedback;feedback("drop")
            check("interaction feedback coalesces simultaneous events",tick>0&&lastFeedback==tick)
            preferences.interactionFeedback=false;feedback("selection");check("feedback respects the disabled preference",lastFeedback==tick);preferences.interactionFeedback=true
            pause()
            _=try await request(["action":"favorite","id":first,"value":true])
            check("favorite journal is durable",outbox.contains{$0["kind"] as? String=="favorite"})
            _=try await request(["action":"playlist","name":"Offline test sequence","track_ids":[first,last,first]])
            check("manual playlist preserves duplicates",(collections.last?["track_ids"] as? [NSNumber])?.map(\.int64Value)==[first,last,first])
            if let id=collections.last?["id"] as? String{do{_=try await request(["action":"playlist","id":id,"base_revision":"stale","name":"Should not save","track_ids":[first]]);check("stale playlist draft is rejected",false)}catch{check("stale playlist draft is rejected",collections.last?["name"] as? String=="Offline test sequence")}}
            if let firstIndex=library.firstIndex(where:{number($0["id"])==first}),let lastIndex=library.firstIndex(where:{number($0["id"])==last}){
                library[firstIndex]["album_id"]="mix-a";library[lastIndex]["album_id"]="mix-b";preferences.automix=true
                _=try await request(["action":"play","ids":[first,last]])
                _=try await request(["action":"seek","position":10.2]);try await Task.sleep(nanoseconds:2_300_000_000)
                check("Automix publishes the dominant incoming entry once",queueIndex==1 && currentID==last && player.currentTime().seconds>0.2)
                pause();check("Automix pause releases both temporary decks",mixTask==nil && preparingMix==nil && outgoingMix==nil && preparedMix==nil && player.rate==0)
                preferences.automix=false
            }
            let lyric=try await request(["action":"lyrics","id":first]) as? [String:Any]
            check("offline lyric document is reused",lyric?["state"] as? String=="ready")
            try persist()
            check("lock screen title follows current song",MPNowPlayingInfoCenter.default().nowPlayingInfo?[MPMediaItemPropertyTitle] as? String==current?["title"] as? String)
            if let baseText = ProcessInfo.processInfo.environment["MUSICOS_TEST_SYNC_BASE"],
               let base = URL(string:baseText), let port = base.port,
               let pin = ProcessInfo.processInfo.environment["MUSICOS_TEST_SYNC_PIN"],
               let firstHash = library.first?["sha256"] as? String {
                let savedLibrary = library, savedCollections = collections, savedConnection = connection
                let backgroundPreference = preferences.backgroundSync
                let fixtureAudio = root.appendingPathComponent("audio/\(firstHash)")
                try? FileManager.default.removeItem(at:fixtureAudio)
                defer {
                    library = savedLibrary; collections = savedCollections; connection = savedConnection
                    preferences.backgroundSync = backgroundPreference
                    status = ["state":"idle"]
                    try? FileManager.default.removeItem(at:fixtureAudio)
                    try? persist()
                }
                connection = Connection(host:"127.0.0.1",port:port,server:"isolated-simulator-fixture",fingerprint:pin,client:"fixture-client",token:"fixture-token")
                preferences.backgroundSync = false
                _ = try await request(["action":"sync"])
                await syncTask?.value
                let firstStatus = try await request(["action":"sync_status"]) as? [String:Any]
                check("sync status is small and reports one verified download",
                    (firstStatus?["sync"] as? [String:Any])?["completed"] as? Int == 1 && firstStatus?["tracks"] == nil &&
                    library.contains(where:{ number($0["id"]) == 9001 }) && (try? hashFileNonisolated(fixtureAudio)) == firstHash)
                let partial = collections.first { $0["id"] as? String == "partial-sync-fixture" }
                check("partly downloaded collection keeps the complete saved sequence",
                    partial?["complete"] as? Bool == false && (partial?["track_ids"] as? [Int64]) == [9001,9002,9001])
                _ = try await request(["action":"sync"])
                await syncTask?.value
                check("repeat sync reports no songs to copy",status["state"] as? String == "complete" && status["total"] as? Int == 0)
                _ = try await request(["action":"sync"])
                for _ in 0..<30 {
                    if status["state"] as? String == "preparing" { break }
                    try await Task.sleep(nanoseconds:30_000_000)
                }
                let wasPreparing = status["state"] as? String == "preparing"
                _ = try await request(["action":"cancel"])
                check("stopping preparation preserves verified songs",wasPreparing && status["state"] as? String == "cancelled" && syncTask == nil &&
                    FileManager.default.fileExists(atPath:fixtureAudio.path))
                let date = Date().addingTimeInterval(-120)
                func sample(_ tick: Int, _ position: Double, _ playing: Bool = true, _ entry: Int64 = 11) -> ListeningSample {
                    ListeningSample(entry:entry,trackID:9001,position:position,duration:60,playing:playing,
                                    uptime:Double(tick),date:date.addingTimeInterval(Double(tick)))
                }
                var occurrence = ListeningOccurrence()
                for second in 0...20 { _ = occurrence.advance(sample(second,Double(second)),server:connection!.server) }
                let skipped = occurrence.advance(sample(21,38),server:connection!.server)
                _ = occurrence.advance(sample(22,38,false),server:connection!.server)
                _ = occurrence.advance(sample(64,38,false),server:connection!.server)
                _ = occurrence.advance(sample(65,38),server:connection!.server)
                var eligible: PhoneListen?
                for second in 1...10 { eligible = occurrence.advance(sample(65+second,Double(38+second)),server:connection!.server) }
                check("phone listens exclude seeks and paused time",skipped == nil && eligible?.listenedSeconds == 30)
                occurrence.queued = true
                check("a duplicate queue entry starts a fresh listen",
                      occurrence.advance(sample(76,0,true,12),server:connection!.server) == nil && !occurrence.queued)

                let relayRoot = root.appendingPathComponent("test-fixtures/listening-relay")
                try FileManager.default.createDirectory(at:relayRoot,withIntermediateDirectories:true)
                try? FileManager.default.removeItem(at:relayRoot.appendingPathComponent("listening-outbox-v1.json"))
                let localRelay = ListeningRelay(root:relayRoot)
                for second in 0...31 {
                    localRelay.update(server:connection?.server,lastfm:true,discord:false,
                        sample:sample(second,Double(second))) { _ in throw URLError(.notConnectedToInternet) }
                }
                check("eligible listens are durable while the Mac is unavailable",localRelay.status["pending"] as? Int == 1)
                localRelay.update(server:nil,lastfm:false,discord:false,sample:nil) { _ in [:] }

                let restarted = ListeningRelay(root:relayRoot)
                let send: ListeningRelay.Sender = { body in try await self.listeningRequest(body) }
                restarted.update(server:connection?.server,lastfm:true,discord:false,sample:nil,send:send)
                for _ in 0..<75 {
                    if restarted.status["pending"] as? Int == 0 { break }
                    try await Task.sleep(nanoseconds:30_000_000)
                }
                check("pinned Wi-Fi relay acknowledges the saved listen after restart",
                      restarted.status["pending"] as? Int == 0 && restarted.status["lastfm_ready"] as? Bool == true)
                let live = ListeningSample(entry:11,trackID:9001,position:3,duration:60,playing:true,
                    uptime:ProcessInfo.processInfo.systemUptime,date:Date())
                restarted.update(server:connection?.server,lastfm:false,discord:true,sample:live,send:send)
                // Pausing while the first request is in flight still sends the clear.
                let paused = ListeningSample(entry:11,trackID:9001,position:3,duration:60,playing:false,
                    uptime:ProcessInfo.processInfo.systemUptime,date:Date())
                restarted.update(server:connection?.server,lastfm:false,discord:true,sample:paused,send:send)
                try await Task.sleep(nanoseconds:450_000_000)
                check("phone relay keeps the two service settings private by default",
                      !MobilePreferences().lastfmListening && !MobilePreferences().discordPresence)
            }
        }catch{checks.append(["name":"native service execution","passed":false,"error":error.localizedDescription])}
        pause()
        try? JSONSerialization.data(withJSONObject:["checks":checks],options:[.prettyPrinted,.sortedKeys]).write(to:root.appendingPathComponent("native-checks.json"),options:.atomic)
    }
}
#endif
