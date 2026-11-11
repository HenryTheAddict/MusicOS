import Foundation

struct MobilePreferences {
    var appearance = "system"
    var colors = "artwork"
    var animations = true
    var interactionFeedback = true
    var musicHaptics = true
    var automix = false
    var backgroundSync = true
    var liveSync = true
    var autoSync = false
    var lastfmListening = false
    var discordPresence = false

    init(_ values: [String: Any] = [:]) {
        if let value = values["appearance"] as? String, ["system", "light", "dark"].contains(value) { appearance = value }
        if let value = values["colors"] as? String, ["artwork", "blue"].contains(value) { colors = value }
        animations = values["animations"] as? Bool ?? true
        interactionFeedback = values["interactionFeedback"] as? Bool ?? true
        musicHaptics = values["musicHaptics"] as? Bool ?? true
        automix = values["automix"] as? Bool ?? false
        backgroundSync = values["backgroundSync"] as? Bool ?? true
        liveSync = values["liveSync"] as? Bool ?? true
        autoSync = values["autoSync"] as? Bool ?? false
        lastfmListening = values["lastfmListening"] as? Bool ?? false
        discordPresence = values["discordPresence"] as? Bool ?? false
    }
    var json: [String: Any] {
        ["appearance": appearance, "colors": colors, "animations": animations,
         "interactionFeedback": interactionFeedback, "musicHaptics": musicHaptics,
         "automix": automix, "backgroundSync": backgroundSync, "liveSync": liveSync, "autoSync": autoSync,
         "lastfmListening": lastfmListening, "discordPresence": discordPresence]
    }
    mutating func set(_ key: String, _ value: Any) throws {
        guard json[key] != nil else { throw NSError(domain: "musicOS", code: 1, userInfo: [NSLocalizedDescriptionKey: "This setting is unavailable"]) }
        var next = json; next[key] = value; self = MobilePreferences(next)
    }
}
