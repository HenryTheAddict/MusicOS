import Foundation
import UIKit

enum ListeningConnectionIssue: Error {
    case unpaired, approval, updateMac
}

struct ListeningSample {
    let entry: Int64
    let trackID: Int64
    let position: Double
    let duration: Double
    let playing: Bool
    let uptime: TimeInterval
    let date: Date
}

struct PhoneListen: Codable {
    let id: String
    let server: String
    let trackID: Int64
    let startedAt: Int64
    let listenedSeconds: Int

    var payload: [String: Any] {
        ["id": id, "track_id": trackID, "started_at": startedAt, "listened_seconds": listenedSeconds]
    }
}

struct ListeningOccurrence {
    private var previous: ListeningSample?
    private var startedAt: Int64?
    private var listened: TimeInterval = 0
    var queued = false

    mutating func advance(_ sample: ListeningSample, server: String) -> PhoneListen? {
        if previous?.entry != sample.entry || previous?.trackID != sample.trackID {
            self = ListeningOccurrence()
        }
        if sample.playing {
            if startedAt == nil { startedAt = Int64(sample.date.timeIntervalSince1970) }
            if let prior = previous, prior.playing {
                let elapsed = sample.uptime - prior.uptime
                let moved = sample.position - prior.position
                // Count time that actually played, excluding seeks, buffering and suspended gaps.
                if elapsed > 0, elapsed <= 1.5, moved > 0, abs(moved - elapsed) <= 0.7 {
                    listened += min(moved, elapsed)
                }
            }
        }
        previous = sample
        guard !queued, sample.duration > 30, listened >= min(sample.duration / 2, 240),
              let startedAt = startedAt else { return nil }
        return PhoneListen(id: UUID().uuidString, server: server, trackID: sample.trackID,
                           startedAt: startedAt, listenedSeconds: Int(listened.rounded(.up)))
    }
}

/// Keeps the offline queue on the phone. Service credentials and publishing stay on the Mac.
@MainActor final class ListeningRelay {
    typealias Sender = ([String: Any]) async throws -> [String: Any]

    private struct Presence: Equatable {
        let entry: Int64
        let track: Int64
        let position: Double
        let time: Int64

        var payload: [String: Any] { ["track_id": track, "position": position, "observed_at": time] }
    }

    private let path: URL
    private var pending: [PhoneListen]
    private var occurrence = ListeningOccurrence()
    private var server: String?
    private var sender: Sender?
    private var desired: Presence?
    private var published: Presence?
    private var listeningEnabled = false
    private var inFlight = false
    private var retry: Task<Void, Never>?
    private var nextAttempt = TimeInterval.zero
    private var lastReport = -TimeInterval.infinity
    private var state = "unchecked"
    private var lastfmReady = false
    private var discordReady = false
    private var storageError = false

    init(root: URL) {
        path = root.appendingPathComponent("listening-outbox-v1.json")
        pending = (try? JSONDecoder().decode([PhoneListen].self, from: Data(contentsOf: path))) ?? []
    }

    var status: [String: Any] {
        ["state": server == nil ? "unpaired" : state, "lastfm_ready": lastfmReady, "discord_ready": discordReady,
         "pending": pending.filter { $0.server == server }.count, "storage_error": storageError]
    }

    func update(server nextServer: String?, lastfm: Bool, discord: Bool, sample: ListeningSample?, send: @escaping Sender) {
        if nextServer != server {
            server = nextServer
            occurrence = ListeningOccurrence()
            desired = nil
            published = nil
            retry?.cancel()
            retry = nil
            nextAttempt = 0
            state = "unchecked"
            lastfmReady = false
            discordReady = false
        }
        sender = send
        listeningEnabled = lastfm
        if let server = server, let sample = sample, lastfm,
           let listen = occurrence.advance(sample, server: server) {
            let cutoff = listen.startedAt - 14 * 24 * 3600
            let retained = pending.filter { $0.startedAt >= cutoff }
            if retained.count < 1000, replaceQueue(retained + [listen]) { occurrence.queued = true }
        }
        desired = nil
        if discord, let sample = sample, sample.playing, sample.position.isFinite, sample.position >= 0 {
            desired = Presence(entry: sample.entry, track: sample.trackID, position: sample.position,
                               time: Int64(sample.date.timeIntervalSince1970))
        }
        pump()
    }

    func resetOccurrence() { occurrence = ListeningOccurrence() }

    func stopCollecting() {
        resetOccurrence()
        _ = replaceQueue([])
    }

    func updateStatus(_ result: [String: Any]) {
        lastfmReady = result["lastfm_ready"] as? Bool == true
        discordReady = result["discord_ready"] as? Bool == true
        state = result["allowed"] as? Bool == false ? "approval" : "connected"
    }

    func failed(_ error: Error) {
        switch error {
        case ListeningConnectionIssue.unpaired: state = "unpaired"
        case ListeningConnectionIssue.approval: state = "approval"
        case ListeningConnectionIssue.updateMac: state = "update_mac"
        default: state = "offline"
        }
        lastfmReady = false
        discordReady = false
    }

    func tryAgain() {
        nextAttempt = 0
        retry?.cancel()
        retry = nil
        pump()
    }

    private func replaceQueue(_ next: [PhoneListen]) -> Bool {
        do {
            try JSONEncoder().encode(next).write(to: path, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
            pending = next
            storageError = false
            return true
        } catch {
            storageError = true
            return false
        }
    }

    private func pump() {
        guard !inFlight, let server = server, let sender = sender else { return }
        let now = ProcessInfo.processInfo.systemUptime
        let wallTime = Int64(Date().timeIntervalSince1970)
        let presence = desired.flatMap { wallTime - $0.time < 12 ? $0 : nil }
        if presence == nil, let published = published, wallTime - published.time > 30 {
            self.published = nil
        }
        let changed = presence?.entry != published?.entry || presence?.track != published?.track
        let heartbeat = presence != nil && now - lastReport >= 10
        let batch = listeningEnabled ? Array(pending.lazy.filter { $0.server == server }.prefix(12)) : []
        guard changed || heartbeat || !batch.isEmpty else { return }
        if presence == nil && UIApplication.shared.applicationState == .background && !changed { return }
        if now < nextAttempt {
            if retry == nil && UIApplication.shared.applicationState != .background {
                let delay = UInt64(max(0, nextAttempt - now) * 1_000_000_000)
                retry = Task { [weak self] in
                    try? await Task.sleep(nanoseconds: delay)
                    guard !Task.isCancelled, let self = self else { return }
                    self.retry = nil
                    self.pump()
                }
            }
            return
        }
        inFlight = true
        Task { [weak self] in
            guard let self = self else { return }
            defer {
                self.inFlight = false
                self.pump()
            }
            do {
                let reply = try await sender(["presence": presence?.payload as Any? ?? NSNull(),
                                              "listens": batch.map(\.payload)])
                guard self.server == server else { return }
                self.updateStatus(reply)
                self.published = presence
                self.lastReport = ProcessInfo.processInfo.systemUptime
                let delivered = Set((reply["accepted"] as? [String] ?? []) + (reply["discarded"] as? [String] ?? []))
                let sent = Set(batch.map(\.id)).intersection(delivered)
                if !sent.isEmpty { _ = self.replaceQueue(self.pending.filter { !sent.contains($0.id) }) }
                // A disabled or full Mac queue is checked again later without hammering the server.
                let blocked = !batch.isEmpty && (sent.isEmpty || self.storageError)
                self.nextAttempt = self.lastReport + (blocked ? (presence == nil ? 30 : 10) : 0)
            } catch {
                guard self.server == server else { return }
                self.failed(error)
                self.nextAttempt = ProcessInfo.processInfo.systemUptime + 25
            }
        }
    }
}
