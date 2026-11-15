import CryptoKit
import Foundation

/// The selected Files directory is external. Credentials, pending Mac edits,
/// device settings, and playback state never enter this catalogue.
enum CloudLibrary {
    struct Failure: LocalizedError {
        let message: String
        var errorDescription: String? { message }
        init(_ message: String) { self.message = message }
    }

    struct Asset: Codable, Hashable, Sendable {
        let kind: String
        let hash: String
        let bytes: Int64
        var name: String { "\(kind)-\(hash)" }

        var valid: Bool {
            ["audio", "artwork", "lyrics"].contains(kind) && bytes >= 0 &&
            hash.utf8.count == 64 && hash.utf8.allSatisfy { (48...57).contains($0) || (97...102).contains($0) }
        }
    }

    struct Manifest: Codable, Sendable {
        let version: Int
        let device: UUID
        let name: String
        let updated: Date
        let songs: Int
        let assets: [Asset]
        let catalogue: Data
    }

    static let documentLimit = 24 * 1024 * 1024
    private static let manager = FileManager.default
    static let stableKeys = ["id", "server_id", "cloud_origin", "cloud_id", "title", "artist", "album", "album_artist",
        "album_id", "year", "genre", "disc_number", "track_number", "duration_ms", "mime", "favorited", "play_count", "isrc", "bpm", "lyrics_edited"]
    private static let collectionKeys = ["id", "name", "kind", "description", "track_ids", "revision",
        "artwork_choice", "cloud_source", "cloud_revision", "complete"]

    // Coordination runs on a utility task. A coordinated package is never held
    // open while editing or playing local files.
    private static func coordinate<T>(_ url: URL, writing: Bool, _ body: (URL) throws -> T) throws -> T {
        var error: NSError?
        var result: Result<T, Error>?
        let accessor: (URL) -> Void = { path in result = Result { try body(path) } }
        let coordinator = NSFileCoordinator()
        if writing { coordinator.coordinate(writingItemAt: url, options: .forReplacing, error: &error, byAccessor: accessor) }
        else { coordinator.coordinate(readingItemAt: url, options: .withoutChanges, error: &error, byAccessor: accessor) }
        if let error { throw error }
        guard let result else { throw Failure("The Files provider could not open the selected folder.") }
        return try result.get()
    }

    static func withFolder<T>(_ bookmark: Data, _ work: (URL) throws -> T) throws -> T {
        var stale = false
        let folder = try URL(resolvingBookmarkData: bookmark, options: [], bookmarkDataIsStale: &stale)
        guard !stale, folder.startAccessingSecurityScopedResource() else {
            throw Failure("Folder access expired. Choose the iCloud Drive folder again.")
        }
        defer { folder.stopAccessingSecurityScopedResource() }
        return try work(folder.appendingPathComponent("musicOS Library", isDirectory: true))
    }

    static func encoded(_ value: [String: Any]) throws -> Data {
        try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    }
    static func decoded(_ data: Data) throws -> [String: Any] {
        guard data.count <= documentLimit, let value = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw Failure("The iCloud library data is invalid or too large.")
        }
        return value
    }

    static func fingerprint(_ path: URL, kind: String) throws -> Asset {
        guard try path.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey]).isRegularFile == true,
              try path.resourceValues(forKeys: [.isSymbolicLinkKey]).isSymbolicLink != true else {
            throw Failure("A library asset is unavailable.")
        }
        let file = try FileHandle(forReadingFrom: path)
        defer { try? file.close() }
        var hash = SHA256(), bytes: Int64 = 0
        while let chunk = try file.read(upToCount: 1024 * 1024), !chunk.isEmpty {
            try Task.checkCancellation()
            bytes += Int64(chunk.count)
            hash.update(data: chunk)
        }
        return Asset(kind: kind, hash: hash.finalize().map { String(format: "%02x", $0) }.joined(), bytes: bytes)
    }

    private static func readManifest(_ file: URL, device: UUID) throws -> Manifest {
        if manager.isUbiquitousItem(at: file) { try? manager.startDownloadingUbiquitousItem(at: file) }
        return try coordinate(file, writing: false) { url in
            let info = try url.resourceValues(forKeys: [.fileSizeKey, .isSymbolicLinkKey])
            guard info.isSymbolicLink != true, let size = info.fileSize, size <= documentLimit else {
                throw Failure("This iCloud library index is invalid or too large.")
            }
            let manifest = try JSONDecoder().decode(Manifest.self, from: Data(contentsOf: url))
            guard manifest.version == 1, manifest.device == device, manifest.songs >= 0,
                  manifest.assets.count <= 100_000, manifest.assets.allSatisfy(\.valid),
                  Set(manifest.assets.map(\.name)).count == manifest.assets.count else {
                throw Failure("This iCloud library index is unsupported.")
            }
            return manifest
        }
    }

    static func index(_ folder: URL) throws -> [Manifest] {
        guard manager.fileExists(atPath: folder.path) || manager.isUbiquitousItem(at: folder) else { return [] }
        let files = try coordinate(folder, writing: false) {
            try manager.contentsOfDirectory(at: $0, includingPropertiesForKeys: [.isSymbolicLinkKey], options: .skipsHiddenFiles)
        }
        var result = [Manifest]()
        for file in files where file.pathExtension == "json" {
            try Task.checkCancellation()
            guard let device = UUID(uuidString: file.deletingPathExtension().lastPathComponent) else { continue }
            result.append(try readManifest(file, device: device))
        }
        return result.sorted { $0.updated > $1.updated }
    }

    private static func only(_ row: [String: Any], _ keys: [String]) -> [String: Any] {
        row.filter { keys.contains($0.key) }
    }

    static func upload(local: URL, catalogue: Data, to folder: URL, device: UUID, name: String) throws -> Manifest {
        let state = try decoded(catalogue)
        let blobs = try coordinate(folder, writing: true) { path in
            let contents = path.appendingPathComponent("Files", isDirectory: true)
            try manager.createDirectory(at: contents, withIntermediateDirectories: true)
            return contents
        }
        let localPath = local.resolvingSymlinksInPath().path
        var published = [String: Asset]()
        var reused = [URL: Asset]()
        func asset(_ path: Any?, _ kind: String, knownHash: String? = nil) throws -> String? {
            guard let path = path as? String else { return nil }
            let source = URL(fileURLWithPath: path).standardizedFileURL.resolvingSymlinksInPath()
            guard source.path.hasPrefix(localPath + "/") else {
                throw Failure("A library file is outside musicOS storage.")
            }
            try Task.checkCancellation()
            let fingerprint: Asset
            if let prior = reused[source], prior.kind == kind {
                fingerprint = prior
            } else {
                let properties = try source.resourceValues(forKeys: [.fileSizeKey, .isRegularFileKey])
                let cached = Asset(kind: kind, hash: knownHash ?? "", bytes: Int64(properties.fileSize ?? -1))
                if source.path == localPath + "/audio/" + cached.hash, properties.isRegularFile == true, cached.valid {
                    // Wi-Fi and iCloud verify immutable, hash-named audio before publishing it.
                    // A new cloud blob still gets verified by copyVerified.
                    fingerprint = cached
                } else {
                    fingerprint = try self.fingerprint(source, kind: kind)
                }
                reused[source] = fingerprint
            }
            if published[fingerprint.name] == nil {
                let destination = blobs.appendingPathComponent(fingerprint.name)
                try coordinate(destination, writing: true) { target in
                    if manager.fileExists(atPath: target.path) || manager.isUbiquitousItem(at: target) { return }
                    // A cancellable verified copy is staged in the app container.
                    try copyVerified(source, target, expected: fingerprint, staging: local)
                }
                published[fingerprint.name] = fingerprint
            }
            return fingerprint.hash
        }

        let rows = try (state["tracks"] as? [[String: Any]] ?? []).map { track in
            var row = only(track, stableKeys)
            guard let audio = try asset(track["path"], "audio", knownHash: track["sha256"] as? String) else { throw Failure("A downloaded song is missing.") }
            row["sha256"] = audio
            row["artwork_sha256"] = try asset(track["artwork_path"], "artwork")
            row["lyrics_sha256"] = try asset(track["lyrics_path"], "lyrics")
            return row
        }
        let collections = try (state["collections"] as? [[String: Any]] ?? []).map { collection in
            var row = only(collection, collectionKeys)
            row["artwork_sha256"] = try asset(collection["artwork_path"], "artwork")
            return row
        }
        var portraits = [String: String]()
        for (name, path) in state["portraits"] as? [String: String] ?? [:] {
            portraits[name] = try asset(path, "artwork")
        }
        let data = try encoded(["tracks": rows, "collections": collections, "portraits": portraits])
        let manifest = Manifest(version: 1, device: device, name: name, updated: Date(), songs: rows.count,
                                assets: published.values.sorted { $0.name < $1.name }, catalogue: data)
        let remote = folder.appendingPathComponent(device.uuidString).appendingPathExtension("json")
        if manager.fileExists(atPath: remote.path), let old = try? readManifest(remote, device: device),
           old.catalogue == manifest.catalogue, old.assets == manifest.assets { return old }
        let bytes = try JSONEncoder().encode(manifest)
        guard bytes.count <= documentLimit else { throw Failure("This library is too large for an iCloud index.") }
        try Task.checkCancellation()
        try coordinate(remote, writing: true) { try bytes.write(to: $0, options: .atomic) }
        return manifest
    }

    static func download(_ folder: URL, local: URL, device: UUID, updated: Date) throws -> Data {
        let manifest = try readManifest(folder.appendingPathComponent(device.uuidString).appendingPathExtension("json"), device: device)
        guard abs(manifest.updated.timeIntervalSince1970 - updated.timeIntervalSince1970) < 0.001 else { throw Failure("The library copy changed. Refresh the list before importing.") }
        let state = try decoded(manifest.catalogue)
        let assets = Dictionary(uniqueKeysWithValues: manifest.assets.map { ($0.name, $0) })
        let blobs = folder.appendingPathComponent("Files", isDirectory: true)
        var resolved = [String: String]()
        func fetch(_ hash: Any?, kind: String) throws -> String? {
            guard let hash else { return nil }
            guard let value = hash as? String, let descriptor = assets["\(kind)-\(value)"] else {
                throw Failure("The iCloud library references a missing file.")
            }
            if let cached = resolved[descriptor.name] { return cached }
            try Task.checkCancellation()
            let destination = local.appendingPathComponent(kind, isDirectory: true).appendingPathComponent(value)
            if (try? fingerprint(destination, kind: kind)) != descriptor {
                let source = blobs.appendingPathComponent(descriptor.name)
                if manager.isUbiquitousItem(at: source) { try? manager.startDownloadingUbiquitousItem(at: source) }
                try coordinate(source, writing: false) { try copyVerified($0, destination, expected: descriptor, staging: local) }
            }
            resolved[descriptor.name] = destination.path
            return destination.path
        }
        guard let tracks = state["tracks"] as? [[String: Any]], tracks.count == manifest.songs else {
            throw Failure("The iCloud library index is incomplete.")
        }
        var ids = Set<Int64>()
        let rows = try tracks.map { track in
            var row = only(track, stableKeys)
            guard let id = (row["id"] as? NSNumber)?.int64Value, id > 0, id <= 9_007_199_254_740_991,
                  ids.insert(id).inserted, row["title"] is String, row["artist"] is String, row["album"] is String,
                  let hash = track["sha256"] as? String, let descriptor = assets["audio-\(hash)"],
                  let path = try fetch(hash, kind: "audio") else {
                throw Failure("A song in the iCloud library is invalid.")
            }
            row["path"] = path
            row["sha256"] = hash
            row["size"] = descriptor.bytes
            row["artwork_path"] = try fetch(track["artwork_sha256"], kind: "artwork")
            row["lyrics_path"] = try fetch(track["lyrics_sha256"], kind: "lyrics")
            return row
        }
        let collections = try (state["collections"] as? [[String: Any]] ?? []).map { collection in
            var row = only(collection, collectionKeys)
            row["artwork_path"] = try fetch(collection["artwork_sha256"], kind: "artwork")
            return row
        }
        var portraits = [String: String]()
        for (artist, hash) in state["portraits"] as? [String: String] ?? [:] {
            portraits[artist] = try fetch(hash, kind: "artwork")
        }
        return try encoded(["tracks": rows, "collections": collections, "portraits": portraits])
    }

    private static func copyVerified(_ source: URL, _ target: URL, expected: Asset, staging: URL) throws {
        try manager.createDirectory(at: target.deletingLastPathComponent(), withIntermediateDirectories: true)
        let temp = staging.appendingPathComponent("cloud-\(UUID().uuidString).partial")
        defer { try? manager.removeItem(at: temp) }
        try manager.copyItem(at: source, to: temp)
        guard try fingerprint(temp, kind: expected.kind) == expected else {
            throw Failure("An iCloud file did not match its library index. Wait for iCloud Drive to finish and try again.")
        }
        try Task.checkCancellation()
        // Use the destination volume when publishing to external File Providers.
        let staged = target.deletingLastPathComponent().appendingPathComponent(".musicOS-\(UUID().uuidString).partial")
        defer { try? manager.removeItem(at: staged) }
        try manager.copyItem(at: temp, to: staged)
        if manager.fileExists(atPath: target.path) { _ = try manager.replaceItemAt(target, withItemAt: staged) }
        else { try manager.moveItem(at: staged, to: target) }
    }
}
