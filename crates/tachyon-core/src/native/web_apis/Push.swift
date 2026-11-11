final class Push {
    private static let api = "push"

    func supported() throws -> Bool {
        try TacWebHost.flag(Self.api, "supported")
    }

    func subscribe(applicationServerKey: String) throws {
        _ = try TacWebHost.call(Self.api, "subscribe", [applicationServerKey])
    }

    func subscription() throws {
        _ = try TacWebHost.call(Self.api, "subscription")
    }

    func unsubscribe() throws {
        _ = try TacWebHost.call(Self.api, "unsubscribe")
    }
}
