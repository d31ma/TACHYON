final class Recorder {
    private static let api = "recorder"

    func supported() throws -> Bool {
        try TacWebHost.flag(Self.api, "supported")
    }

    func start(recording: String, audioTracks: String, videoTrack: String, fileName: String, format: String) throws {
        _ = try TacWebHost.call(Self.api, "start", [recording, audioTracks, videoTrack, fileName, format])
    }

    func addAudio(recording: String, track: String) throws {
        _ = try TacWebHost.call(Self.api, "addAudio", [recording, track])
    }

    func removeAudio(recording: String, track: String) throws {
        _ = try TacWebHost.call(Self.api, "removeAudio", [recording, track])
    }

    func showVideo(recording: String, track: String) throws {
        _ = try TacWebHost.call(Self.api, "showVideo", [recording, track])
    }

    func pause(recording: String) throws {
        _ = try TacWebHost.call(Self.api, "pause", [recording])
    }

    func resume(recording: String) throws {
        _ = try TacWebHost.call(Self.api, "resume", [recording])
    }

    func stop(recording: String) throws {
        _ = try TacWebHost.call(Self.api, "stop", [recording])
    }
}
