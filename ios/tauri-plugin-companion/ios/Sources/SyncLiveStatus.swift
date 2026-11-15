import Foundation
import UIKit
import ActivityKit
import MusicOSActivityShared

@MainActor final class SyncLiveStatus {
    static let shared = SyncLiveStatus()
    private var pending: Task<Void, Never>?
    private struct Update: Equatable {
        let phase: String
        let message: String
        let completed: Int
        let total: Int
        let jobID:String?
    }
    private var next: (Update, Bool)?
    private var last: Update?
    private var enabled = true

    var supported: Bool {
        guard #available(iOS 16.2,*), let folder = Bundle.main.builtInPlugInsURL else { return false }
        return FileManager.default.fileExists(atPath:folder.appendingPathComponent("MusicOSSyncWidget.appex").path)
    }

    func update(_ status: [String: Any], enabled: Bool) {
        guard #available(iOS 16.2,*), supported else { return }
        self.enabled = enabled
        let phase = status["state"] as? String ?? "idle"
        let terminal = ["complete","cancelled","error","approval","idle"].contains(phase)
        let state = Update(
            phase:phase, message:String((status["message"] as? String ?? "").prefix(120)),
            completed:status["completed"] as? Int ?? 0, total:status["total"] as? Int ?? 0,
            jobID:status["job_id"] as? String)
        next = (state, terminal)
        if terminal || !enabled { pending?.cancel(); pending = nil }
        guard pending == nil else { return }
        pending = Task {
            if !terminal && enabled { try? await Task.sleep(nanoseconds:350_000_000) }
            guard !Task.isCancelled, let (state, terminal) = next else { return }
            pending = nil
            next = nil
            let activities = Activity<SyncActivityAttributes>.activities
            guard self.enabled else {
                for activity in activities { await activity.end(nil,dismissalPolicy:.immediate) }
                last = nil
                return
            }
            let content = ActivityContent(state:SyncActivityAttributes.ContentState(phase:state.phase,message:state.message,completed:state.completed,total:state.total,jobID:state.jobID), staleDate:terminal ? nil : Date().addingTimeInterval(state.jobID==nil ? 180 : 90))
            if terminal {
                for activity in activities {
                    await activity.end(content,dismissalPolicy:state.phase == "idle" ? .immediate : .after(Date().addingTimeInterval(20)))
                }
                last = nil
            } else if let activity = activities.first {
                if last != state { last = state; await activity.update(content) }
            } else if UIApplication.shared.applicationState == .active && ActivityAuthorizationInfo().areActivitiesEnabled {
                if (try? Activity.request(attributes:SyncActivityAttributes(computer:"Computer"),content:content,pushType:nil)) != nil { last = state }
            }
        }
    }
}
