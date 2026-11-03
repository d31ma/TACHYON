final class Notifications {
    private static let api = "notifications"

    func permission() throws -> String {
        try TacWebHost.text(Self.api, "permission")
    }

    func requestPermission() throws {
        _ = try TacWebHost.call(Self.api, "requestPermission")
    }

    func show(title: String, body: String) throws {
        _ = try TacWebHost.call(Self.api, "show", [title, body])
    }
}
