import Tauri
import UIKit
import AVFoundation
import MediaPlayer
import AVKit
import WebKit
import CryptoKit
import Security

@_silgen_name("musicos_prepare_audio")
private func prepareAudio(_ source: UnsafePointer<CChar>, _ destination: UnsafePointer<CChar>) -> Int32

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
    let discovery=MacDiscovery();var discoveryStop:Task<Void,Never>?
    var connection = Credentials.load()
    var library: [[String:Any]] = []
    var collections: [[String:Any]] = []
    var portraits:[String:String]=[:]
    var libraryChanged:(()->Void)?
    var outbox: [[String:Any]] = []
    var conflicts: [[String:Any]] = []
    var status: [String:Any] = ["state":"idle", "completed":0, "total":0]
    var queue: [Int64] = []; var queueIndex = 0
    var queueEntryIds:[Int64]=[];var entryCounter:Int64=Int64(Date().timeIntervalSince1970*1000);var repeatMode="none";var shuffled=false;var originalQueue:[(Int64,Int64)]=[]
    var wantsPlayback=false;var preparing=false;var decodedJobs:[String:Task<Int32,Never>]=[:];var savedPosition=0.0;var lastSaved=0.0
    var cachedArtPath:String?;var cachedArtwork:MPMediaItemArtwork?
    var player = AVPlayer(); var observer: Any?; var notifications: [NSObjectProtocol] = []
    var emit: ((String)->Void)?
    var syncTask: Task<Void,Never>?; var prepareTask: Task<Void,Never>?
    var generation = 0; var resumeAfterInterruption = false; var playbackError: String?
    var currentID: Int64? { queue.indices.contains(queueIndex) ? queue[queueIndex] : nil }
    var current: [String:Any]? { library.first { number($0["id"]) == currentID } }
    func number(_ value: Any?) -> Int64 { (value as? NSNumber)?.int64Value ?? -1 }
    init() {
        root = FileManager.default.urls(for:.applicationSupportDirectory, in:.userDomainMask)[0].appendingPathComponent("musicOS", isDirectory:true)
        try? FileManager.default.createDirectory(at:root, withIntermediateDirectories:true)
        for name in ["audio", "artwork", "lyrics", "decoded", "partial"] { try? FileManager.default.createDirectory(at:root.appendingPathComponent(name), withIntermediateDirectories:true) }
        if let data = try? Data(contentsOf:root.appendingPathComponent("library.json")), let state = try? JSONSerialization.jsonObject(with:data) as? [String:Any] {
            portraits=state["portraits"] as? [String:String] ?? [:];library = state["tracks"] as? [[String:Any]] ?? []; collections = state["collections"] as? [[String:Any]] ?? []
            outbox = state["outbox"] as? [[String:Any]] ?? []; conflicts = state["conflicts"] as? [[String:Any]] ?? []
            queue = (state["queue"] as? [NSNumber] ?? []).map(\.int64Value); queueIndex = state["queue_index"] as? Int ?? 0
            savedPosition=state["position"] as? Double ?? 0;repeatMode=state["repeat"] as? String ?? "none"
            queueEntryIds=(state["queue_entries"] as? [NSNumber] ?? []).map(\.int64Value)
            if queueEntryIds.count != queue.count{queueEntryIds=queue.map{_ in entryCounter+=1;return entryCounter}}
        }
        // iOS can relocate the data container during an update. Persist only library-relative assets.
        library=library.map{restorePaths($0)};collections=collections.map{restorePaths($0)};portraits=portraits.mapValues{restorePath($0)}
        observer = player.addPeriodicTimeObserver(forInterval:CMTime(seconds:0.2, preferredTimescale:600), queue:.main) { [weak self] _ in Task { @MainActor in self?.publish();if let self=self,self.player.rate>0,Date().timeIntervalSince1970-self.lastSaved>10{self.lastSaved=Date().timeIntervalSince1970;try? self.persist()} } }
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
            Task { @MainActor in if reason == AVAudioSession.RouteChangeReason.oldDeviceUnavailable.rawValue { self?.pause() }; self?.publish() }
        })
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
        let value: [String:Any] = ["version":1,"portraits":portraits.mapValues{storePath($0)},"tracks":library.map{storePaths($0)},"collections":collections.map{storePaths($0)},"outbox":outbox,"conflicts":conflicts,"queue":queue,"queue_index":queueIndex,"queue_entries":queueEntryIds,"position":player.currentItem==nil ? savedPosition:max(0,player.currentTime().seconds.isFinite ? player.currentTime().seconds:0),"repeat":repeatMode]
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
    func snapshot() -> [String:Any] { ["portraits":portraits,"nearby_macs":discovery.results,"tracks":library.map(trackPresentation),"collections":collections,"playback":playback(),"sync":status,"paired":connection != nil,"pending_changes":outbox.count,"conflicts":conflicts,"text_scale":UIFont.preferredFont(forTextStyle:.body).pointSize/17,"storage_bytes":library.reduce(Int64(0)) { $0 + number($1["size"]) }] }
    func publish() {
        if let t=current {
            var info: [String:Any] = [MPMediaItemPropertyTitle:t["title"] ?? "",MPMediaItemPropertyArtist:t["artist"] ?? "",MPMediaItemPropertyAlbumTitle:t["album"] ?? "",MPMediaItemPropertyPlaybackDuration:(t["duration_ms"] as? Double ?? 0)/1000,MPNowPlayingInfoPropertyElapsedPlaybackTime:max(0,player.currentTime().seconds.isFinite ? player.currentTime().seconds : 0),MPNowPlayingInfoPropertyPlaybackRate:player.rate]
            let artPath=t["artwork_path"] as? String
            if cachedArtPath != artPath{cachedArtPath=artPath;cachedArtwork=artPath.flatMap{UIImage(contentsOfFile:$0)}.map{image in MPMediaItemArtwork(boundsSize:image.size){_ in image}}}
            if let art=cachedArtwork{info[MPMediaItemPropertyArtwork]=art}
            MPNowPlayingInfoCenter.default().nowPlayingInfo=info
        } else {MPNowPlayingInfoCenter.default().nowPlayingInfo=nil}
        if UIApplication.shared.applicationState != .background { emit?(json(playback())) }
    }
    func loadCurrent() async throws {
        guard let t=current, let path=t["path"] as? String else { throw CompanionError.message("Choose a downloaded song first") }
        generation += 1; let request=generation
        var url=URL(fileURLWithPath:path);preparing=true;defer{preparing=false}
        let native=["audio/mpeg","audio/mp4","audio/aac","audio/wav","audio/x-wav","audio/aiff","audio/x-m4a"]
        if !native.contains(t["mime"] as? String ?? "") {
            let destination=root.appendingPathComponent("decoded/\(t["sha256"] as? String ?? UUID().uuidString).wav")
            let key=destination.path
            let job=decodedJobs[key] ?? Task.detached(priority:.userInitiated) { path.withCString { s in destination.path.withCString { d in prepareAudio(s,d) } } }
            decodedJobs[key]=job;let code=await job.value;decodedJobs.removeValue(forKey:key)
            guard request==generation else { throw CancellationError() }
            guard code==0 else { throw CompanionError.message("This audio file could not be decoded") }; url=destination
        }
        guard request==generation else { return }
        player.replaceCurrentItem(with:AVPlayerItem(url:url));if savedPosition>0{await player.seek(to:CMTime(seconds:savedPosition,preferredTimescale:600),toleranceBefore:.zero,toleranceAfter:.zero)};playbackError=nil;publish()
    }
    func play() async throws {
        wantsPlayback=true
        try AVAudioSession.sharedInstance().setCategory(.playback, mode:.default, options:[.allowAirPlay])
        try AVAudioSession.sharedInstance().setActive(true)
        if player.currentItem==nil { try await loadCurrent() }
        guard wantsPlayback else{return};player.play();publish()
    }
    func pause() {wantsPlayback=false;generation+=1;player.pause();try? persist();publish()}
    func toggle() async throws { if player.rate>0||wantsPlayback { pause() } else { try await play() } }
    func seek(_ seconds: Double) { player.seek(to:CMTime(seconds:max(0,seconds),preferredTimescale:600),toleranceBefore:.zero,toleranceAfter:.zero) { [weak self] _ in Task { @MainActor in self?.publish() } } }
    func skip(_ by:Int,end:Bool=false) async throws {
        guard !queue.isEmpty else { return }
        if end&&repeatMode=="one"{seek(0);try await play();return}
        if by<0 && player.currentTime().seconds>3 { seek(0); return }
        let candidate=queueIndex+by;let next=(candidate>=queue.count&&repeatMode=="all") ? 0:candidate
        guard queue.indices.contains(next) else { if end { pause();seek(0) };return }
        queueIndex=next;savedPosition=0;player.pause();player.replaceCurrentItem(with:nil);try persist();try await play()
    }
    func request(_ payload:[String:Any]) async throws -> Any {
        switch payload["action"] as? String ?? "snapshot" {
        case "snapshot": return snapshot()
        case "playback": return playback()
        case "play":
            if let ids=payload["ids"] as? [NSNumber] { let values=ids.map(\.int64Value); guard !values.isEmpty,values.allSatisfy({ id in library.contains { number($0["id"])==id } }) else { throw CompanionError.message("Some songs have not downloaded yet") };generation+=1;player.pause();player.replaceCurrentItem(with:nil);queue=values;queueEntryIds=values.map{_ in entryCounter+=1;return entryCounter};originalQueue=[];shuffled=false;savedPosition=0;queueIndex=payload["index"] as? Int ?? 0;guard queue.indices.contains(queueIndex) else { throw CompanionError.message("Invalid queue position") };try persist() }
            try await play(); return playback()
        case "toggle": try await toggle(); return playback()
        case "pause": pause(); return playback()
        case "next": try await skip(1);return playback()
        case "previous": try await skip(-1);return playback()
        case "seek":
            let seconds=max(0,min(payload["position"] as? Double ?? 0,(current?["duration_ms"] as? Double ?? 0)/1000)),token=generation
            if player.currentItem != nil{_=await player.seek(to:CMTime(seconds:seconds,preferredTimescale:600),toleranceBefore:.zero,toleranceAfter:.zero)}else{savedPosition=seconds}
            if generation==token{publish()};return playback()
        case "queue_play":
            let index=payload["index"] as? Int ?? -1
            guard queue.indices.contains(index) else {throw CompanionError.message("Queue entry moved")}
            if index != queueIndex{generation+=1;player.pause();player.replaceCurrentItem(with:nil);queueIndex=index;savedPosition=0}
            try await play();try persist();return playback()
        case "queue_move":
            let from=payload["from"] as? Int ?? -1,to=payload["to"] as? Int ?? -1
            guard queue.indices.contains(from),queue.indices.contains(to) else { throw CompanionError.message("Invalid queue position") }
            let currentEntry=queueIndex;let id=queue.remove(at:from),entry=queueEntryIds.remove(at:from);queue.insert(id,at:to);queueEntryIds.insert(entry,at:to)
            if from==currentEntry { queueIndex=to } else if from<currentEntry && to>=currentEntry { queueIndex-=1 } else if from>currentEntry && to<=currentEntry { queueIndex+=1 }
            try persist();return playback()
        case "queue_add":
            let ids=(payload["ids"] as? [NSNumber] ?? []).map(\.int64Value)
            guard !ids.isEmpty,ids.allSatisfy({id in library.contains{number($0["id"])==id}})else{throw CompanionError.message("Download these songs first")}
            let at=payload["next"] as? Bool==true ? min(queueIndex+1,queue.count):queue.count
            queue.insert(contentsOf:ids,at:at);queueEntryIds.insert(contentsOf:ids.map{_ in entryCounter+=1;return entryCounter},at:at);try persist();publish();return playback()
        case "queue_remove":
            let index=payload["index"] as? Int ?? -1;guard queue.indices.contains(index)else{throw CompanionError.message("Queue entry moved")}
            let wasPlaying=player.rate>0;queue.remove(at:index);queueEntryIds.remove(at:index)
            if index<queueIndex{queueIndex-=1}else if index==queueIndex{generation+=1;player.pause();player.replaceCurrentItem(with:nil);savedPosition=0;queueIndex=min(queueIndex,max(0,queue.count-1));if wasPlaying && !queue.isEmpty{try await play()}}
            try persist();publish();return playback()
        case "shuffle":
            let entries=Array(zip(queueEntryIds,queue));if !shuffled{originalQueue=entries;let start=min(queueIndex+1,queue.count);let tail=Array(entries.dropFirst(start)).shuffled();queue=Array(queue.prefix(start))+tail.map{$0.1};queueEntryIds=Array(queueEntryIds.prefix(start))+tail.map{$0.0}}
            else{let start=min(queueIndex+1,queue.count),pending=Set(queueEntryIds.dropFirst(start));let tail=originalQueue.filter{pending.contains($0.0)}+entries.dropFirst(start).filter{entry in !originalQueue.contains{$0.0==entry.0}};queue=Array(queue.prefix(start))+tail.map{$0.1};queueEntryIds=Array(queueEntryIds.prefix(start))+tail.map{$0.0}}
            shuffled.toggle();try persist();publish();return playback()
        case "repeat":repeatMode=repeatMode=="none" ? "all":repeatMode=="all" ? "one":"none";try persist();publish();return playback()
        case "favorite":
            let id=number(payload["id"]),value=payload["value"] as? Bool ?? false
            guard let i=library.firstIndex(where:{number($0["id"])==id}) else { throw CompanionError.message("Song is unavailable") }
            let before=library[i]["favorited"] as? Bool ?? false; library[i]["favorited"]=value
            outbox.append(["id":UUID().uuidString,"kind":"favorite","track_id":id,"base":before,"value":value]);try persist();return snapshot()
        case "playlist":
            let id=payload["id"] as? String ?? "phone/\(UUID().uuidString)"
            let old=collections.first(where:{$0["id"] as? String==id})
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
            guard syncTask==nil else { return status }
            syncTask=Task { do { try await sync() } catch is CancellationError { status=["state":"cancelled","message":"Downloads kept. Sync again to resume."] } catch { status=["state":"error","message":error.localizedDescription] };syncTask=nil;if UIApplication.shared.applicationState != .background{libraryChanged?()} }
            return ["state":"starting"]
        case "cancel": syncTask?.cancel();return ["state":"cancelling"]
        case "disconnect": syncTask?.cancel();connection=nil;try Credentials.save(nil);return snapshot()
        case "resolve":
            let id=payload["id"] as? String ?? "",phone=payload["phone"] as? Bool ?? false
            if let c=conflicts.first(where:{$0["id"] as? String==id}),phone,var op=c["operation"] as? [String:Any] { op["id"]=UUID().uuidString;op["force"]=true;outbox.append(op) }
            if !phone,let conflict=conflicts.first(where:{$0["id"] as? String==id}),let operation=conflict["operation"] as? [String:Any] {
                if operation["kind"] as? String=="favorite",let value=conflict["mac"] as? Bool,let index=library.firstIndex(where:{number($0["id"])==number(operation["track_id"])}){library[index]["favorited"]=value}
                if let row=conflict["mac"] as? [String:Any],let collection=row["id"] as? String{collections.removeAll{$0["id"] as? String==collection};collections.append(row)}
            }
            conflicts.removeAll{$0["id"] as? String==id};try persist();return snapshot()
        case "discover":
            discovery.start();discoveryStop?.cancel();discoveryStop=Task{try? await Task.sleep(nanoseconds:5_000_000_000);if !Task.isCancelled{discovery.stop()}}
            discovery.finish={ [weak self] mac in
                guard let self=self,var connection=self.connection,mac["id"]==connection.server,mac["fingerprint"]==connection.fingerprint,let host=mac["host"],let port=Int(mac["port"] ?? "") else{return}
                connection.host=host;connection.port=port;self.connection=connection;try? Credentials.save(connection)
            }
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
        let delegate=PinnedConnection(c.fingerprint),configuration=URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest=30;configuration.timeoutIntervalForResource=120
        let session=URLSession(configuration:configuration,delegate:delegate,delegateQueue:nil)
        defer{session.invalidateAndCancel()}
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
            try Credentials.save(connection);return result
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
    func sync() async throws {
        status=["state":"connecting","message":"Connecting to your Mac…"]
        let approval=try await remote("pair/status") as? [String:Any]
        guard approval?["state"] as? String=="approved" else {throw CompanionError.message("Approve this iPhone in Devices on your Mac first")}
        status=["state":"preparing","message":"Checking prepared copies…"]
        var attempts=0
        while let progress=try await remote("preparation") as? [String:Any],progress["preparing"] as? Bool==true {
            try Task.checkCancellation();attempts+=1;guard attempts<1500 else{throw CompanionError.message("The Mac is still preparing music. Sync again when it finishes.")}
            status=["state":"preparing","completed":progress["completed"] ?? 0,"total":progress["total"] ?? 0,"message":progress["current"] ?? "Preparing copies on the Mac…"]
            try await Task.sleep(nanoseconds:400_000_000)
        }
        let manifest=try await remote("manifest") as? [String:Any] ?? [:]
        guard manifest["version"] as? Int==1,manifest["server_id"] as? String==connection?.server,let tracks=manifest["tracks"] as? [[String:Any]] else {throw CompanionError.message("This Mac's library format is unavailable")}
        let total=tracks.count
        let required=tracks.reduce(Int64(0)) { n,t in n+(library.contains{ $0["sha256"] as? String==t["sha256"] as? String } ? 0:number(t["size"])) }
        let values=try root.resourceValues(forKeys:[.volumeAvailableCapacityForImportantUsageKey])
        if let free=values.volumeAvailableCapacityForImportantUsage,required>free {throw CompanionError.message("Not enough space. Choose fewer songs on the Mac, then retry.")}
        let pendingFavorites=Set(outbox.filter{$0["kind"] as? String=="favorite"}.map{number($0["track_id"])})
        for (i,var track) in tracks.enumerated() {
            try Task.checkCancellation()
            status=["state":"downloading","completed":i,"total":total,"message":track["title"] ?? "Downloading music"]
            let id=number(track["id"]),hash=track["sha256"] as? String ?? ""
            let path=try await download("tracks/\(id)",hash:hash,size:number(track["size"]),kind:"audio")
            track["path"]=path.path;track["server_id"]=connection!.server
            if let h=track["artwork_sha256"] as? String {track["artwork_path"]=try await download("artwork/\(h)",hash:h,size:nil,kind:"artwork").path}
            if let h=track["lyrics_sha256"] as? String {track["lyrics_path"]=try await download("assets/\(h)",hash:h,size:nil,kind:"lyrics").path}
            if let old=library.first(where:{number($0["id"])==id}) {
                if pendingFavorites.contains(id){track["favorited"]=old["favorited"]}
                if old["lyrics_edited"] as? Bool==true {track["lyrics_path"]=old["lyrics_path"];track["lyrics_edited"]=true}
                track["play_count"]=old["play_count"] ?? 0
            }
            library.removeAll{number($0["id"])==id};library.append(track);try persist()
        }
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
        try persist();_ = try? await remote("sync-complete",body:["revision":manifest["revision"] ?? "","track_count":total])
        status=["state":"complete","completed":total,"total":total,"message":!outbox.isEmpty && connection?.syncChanges==false ? "Ready offline. Update musicOS on the computer and pair again to sync favorites and playlists.":conflicts.isEmpty ? "Ready offline":"Ready offline. Review sync conflicts."]
    }
}
class CompanionPlugin: Plugin {
    override func load(webview:WKWebView){
        #if DEBUG
        if ProcessInfo.processInfo.environment["MUSICOS_COMPANION_SELF_TEST"]=="1"{Task{@MainActor in await Companion.shared.runNativeChecks()}}
        #endif
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
            let relative="test-fixtures/original.flac";check("asset paths survive container relocation",restorePath("/old/Library/Application Support/musicOS/"+relative)==root.appendingPathComponent(relative).path && storePath(root.appendingPathComponent(relative).path)==relative)
            let ids=library.map{number($0["id"])}
            for id in ids {
                _=try await request(["action":"play","ids":[id]])
                try await Task.sleep(nanoseconds:700_000_000)
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
            pause()
            _=try await request(["action":"favorite","id":first,"value":true])
            check("favorite journal is durable",outbox.contains{$0["kind"] as? String=="favorite"})
            _=try await request(["action":"playlist","name":"Offline test sequence","track_ids":[first,last,first]])
            check("manual playlist preserves duplicates",(collections.last?["track_ids"] as? [NSNumber])?.map(\.int64Value)==[first,last,first])
            let lyric=try await request(["action":"lyrics","id":first]) as? [String:Any]
            check("offline lyric document is reused",lyric?["state"] as? String=="ready")
            try persist()
            check("lock screen title follows current song",MPNowPlayingInfoCenter.default().nowPlayingInfo?[MPMediaItemPropertyTitle] as? String==current?["title"] as? String)
        }catch{checks.append(["name":"native service execution","passed":false,"error":error.localizedDescription])}
        pause()
        try? JSONSerialization.data(withJSONObject:["checks":checks],options:[.prettyPrinted,.sortedKeys]).write(to:root.appendingPathComponent("native-checks.json"),options:.atomic)
    }
}
#endif
