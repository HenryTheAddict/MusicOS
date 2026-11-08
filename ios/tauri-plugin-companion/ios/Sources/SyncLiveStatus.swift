import Foundation
import UIKit
import ActivityKit
import MusicOSActivityShared

@MainActor final class SyncLiveStatus {
    static let shared=SyncLiveStatus()
    private var pending:Task<Void,Never>?
    private var generation=0
    var supported:Bool {
        guard #available(iOS 16.2,*),let folder=Bundle.main.builtInPlugInsURL else{return false}
        return ((try? FileManager.default.contentsOfDirectory(at:folder,includingPropertiesForKeys:nil)) ?? []).contains{$0.pathExtension=="appex"}
    }
    func update(_ status:[String:Any],enabled:Bool){
        guard #available(iOS 16.2,*),supported else{return}
        generation+=1;let token=generation;pending?.cancel()
        let phase=status["state"] as? String ?? "idle"
        let terminal=["complete","cancelled","error","idle"].contains(phase)
        let state=SyncActivityAttributes.ContentState(phase:phase,message:String((status["message"] as? String ?? "Copying music…").prefix(120)),completed:(status["completed"] as? Int ?? 0),total:(status["total"] as? Int ?? 0))
        pending=Task{
            if !terminal{try? await Task.sleep(nanoseconds:400_000_000)}
            guard !Task.isCancelled,token==generation else{return}
            var activity=Activity<SyncActivityAttributes>.activities.first
            if !enabled {
                for old in Activity<SyncActivityAttributes>.activities{await old.end(nil,dismissalPolicy:.immediate)}
                return
            }
            if activity==nil,!terminal,UIApplication.shared.applicationState == .active,ActivityAuthorizationInfo().areActivitiesEnabled{
                activity=try? Activity.request(attributes:SyncActivityAttributes(computer:"Computer"),content:ActivityContent(state:state,staleDate:Date().addingTimeInterval(120)),pushType:nil)
            }
            guard let activity=activity else{return}
            let content=ActivityContent(state:state,staleDate:Date().addingTimeInterval(120))
            if terminal{await activity.end(content,dismissalPolicy:.after(Date().addingTimeInterval(15)))}else{await activity.update(content)}
        }
    }
}
