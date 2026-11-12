final class Passkeys {
    private static let api = "passkeys"

    func supported() throws -> Bool {
        try TacWebHost.flag(Self.api, "supported")
    }

    func check() throws {
        _ = try TacWebHost.call(Self.api, "check")
    }

    func create(optionsJson: String) throws {
        _ = try TacWebHost.call(Self.api, "create", [optionsJson])
    }

    func get(optionsJson: String) throws {
        _ = try TacWebHost.call(Self.api, "get", [optionsJson])
    }

    func offer(optionsJson: String) throws {
        _ = try TacWebHost.call(Self.api, "offer", [optionsJson])
    }

    func cancel() throws {
        _ = try TacWebHost.call(Self.api, "cancel")
    }
}
