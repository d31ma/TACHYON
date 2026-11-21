final class Media {
    private static let api = "media"

    func supported(feature: String) throws -> Bool {
        try TacWebHost.answer(TacWebHost.call(Self.api, "supported", [feature]), as: Bool.self)
    }

    func devices(request: String) throws {
        _ = try TacWebHost.call(Self.api, "devices", [request])
    }

    func chooseSpeaker(device: String) throws {
        _ = try TacWebHost.call(Self.api, "chooseSpeaker", [device])
    }

    func captureMicrophone(track: String, device: String, processing: Bool) throws {
        _ = try TacWebHost.call(Self.api, "captureMicrophone", [track, device, processing])
    }

    func captureCamera(track: String, device: String, quality: String) throws {
        _ = try TacWebHost.call(Self.api, "captureCamera", [track, device, quality])
    }

    func captureScreen(track: String, withAudio: Bool) throws {
        _ = try TacWebHost.call(Self.api, "captureScreen", [track, withAudio])
    }

    func stopTrack(track: String) throws {
        _ = try TacWebHost.call(Self.api, "stopTrack", [track])
    }

    func mute(track: String, muted: Bool) throws {
        _ = try TacWebHost.call(Self.api, "mute", [track, muted])
    }

    func useFrameKey(keyId: Int, key: String) throws {
        _ = try TacWebHost.call(Self.api, "useFrameKey", [keyId, key])
    }

    func sendFrameKey(keyId: Int) throws {
        _ = try TacWebHost.call(Self.api, "sendFrameKey", [keyId])
    }

    func forgetFrameKey(keyId: Int) throws {
        _ = try TacWebHost.call(Self.api, "forgetFrameKey", [keyId])
    }

    func encryptFrames(connection: String, on: Bool) throws {
        _ = try TacWebHost.call(Self.api, "encryptFrames", [connection, on])
    }

    func callActive(on: Bool, title: String) throws {
        _ = try TacWebHost.call(Self.api, "callActive", [on, title])
    }

    func send(connection: String, track: String, stream: String, simulcast: Bool) throws {
        _ = try TacWebHost.call(Self.api, "send", [connection, track, stream, simulcast])
    }

    func replace(connection: String, track: String, withTrack: String) throws {
        _ = try TacWebHost.call(Self.api, "replace", [connection, track, withTrack])
    }

    func unsend(connection: String, track: String) throws {
        _ = try TacWebHost.call(Self.api, "unsend", [connection, track])
    }

    func stats(request: String, connection: String) throws {
        _ = try TacWebHost.call(Self.api, "stats", [request, connection])
    }

    func speaking(on: Bool) throws {
        _ = try TacWebHost.call(Self.api, "speaking", [on])
    }

    func resumeAudio() throws {
        _ = try TacWebHost.call(Self.api, "resumeAudio")
    }
}
