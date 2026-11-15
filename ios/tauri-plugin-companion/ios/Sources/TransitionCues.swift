import AVFoundation

@_silgen_name("musicos_transition_cue")
private func selectTransitionCue(
    _ tail: UnsafePointer<Float>, _ tailCount: Int, _ tailStart: Float,
    _ head: UnsafePointer<Float>, _ headCount: Int, _ duration: Float, _ output: UnsafeMutablePointer<Float>
)

struct TransitionCues: Sendable {
    let start: Double
    let overlap: Double
    let entrance: Double

    static func measure(outgoing: URL, incoming: URL, duration: Double) throws -> TransitionCues {
        let tailStart = max(0, duration - 15)
        let tail = (try? envelope(outgoing, start: tailStart, bins: 60)) ?? []
        let head = (try? envelope(incoming, start: 0, bins: 48)) ?? []
        try Task.checkCancellation()
        // A non-null sentinel also works for an empty profile.
        let tailData = tail.isEmpty ? [Float(0)] : tail
        let headData = head.isEmpty ? [Float(0)] : head
        let fallback = min(2, duration / 4)
        var plan = [Float(duration - fallback), Float(fallback), 0]
        tailData.withUnsafeBufferPointer { tailBuffer in
            headData.withUnsafeBufferPointer { headBuffer in
                plan.withUnsafeMutableBufferPointer { output in
                    selectTransitionCue(tailBuffer.baseAddress!, tail.count, Float(tailStart),
                        headBuffer.baseAddress!, head.count, Float(duration), output.baseAddress!)
                }
            }
        }
        return Self(start: Double(plan[0]), overlap: Double(plan[1]), entrance: Double(plan[2]))
    }

    /// One low-resolution RMS bin per quarter second, using only the source ends.
    /// Runs off the player thread. Portable formats use the already decoded file.
    private static func envelope(_ url: URL, start: Double, bins: Int) throws -> [Float] {
        let file = try AVAudioFile(forReading: url, commonFormat: .pcmFormatFloat32, interleaved: false)
        let format = file.processingFormat
        let sampleRate = format.sampleRate
        guard sampleRate.isFinite, (8_000...384_000).contains(sampleRate), format.channelCount > 0 else { return [] }
        let position = AVAudioFramePosition(start * sampleRate)
        guard position < file.length else { return [] }
        file.framePosition = position
        let binLength = AVAudioFrameCount(sampleRate / 4)
        guard let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: binLength) else { return [] }
        var values = [Float]()
        for _ in 0..<bins {
            try Task.checkCancellation()
            guard file.framePosition < file.length else { break }
            try file.read(into: buffer, frameCount: binLength)
            guard buffer.frameLength > 0, let channels = buffer.floatChannelData else { break }
            var power: Float = 0
            var count = 0
            for channel in 0..<min(Int(format.channelCount), 2) {
                for frame in stride(from: 0, to: Int(buffer.frameLength), by: 8) {
                    let value = channels[channel][frame]
                    if value.isFinite { power += value * value; count += 1 }
                }
            }
            values.append((power / Float(max(1, count))).squareRoot())
        }
        return values
    }
}
