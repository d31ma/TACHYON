final class Passkeys {
    private static let api = "passkeys"

    func supported() throws -> Bool {
        try TacWebHost.flag(Self.api, "supported")
    }

    func check() throws {
        _ = try TacWebHost.call(Self.api, "check")
    }

    func create(optionsJson: String, prfKey: String) throws {
        _ = try TacWebHost.call(Self.api, "create", [optionsJson, prfKey])
    }

    func get(optionsJson: String, prfKey: String) throws {
        _ = try TacWebHost.call(Self.api, "get", [optionsJson, prfKey])
    }

    func offer(optionsJson: String, prfKey: String) throws {
        _ = try TacWebHost.call(Self.api, "offer", [optionsJson, prfKey])
    }

    func cancel() throws {
        _ = try TacWebHost.call(Self.api, "cancel")
    }
}
