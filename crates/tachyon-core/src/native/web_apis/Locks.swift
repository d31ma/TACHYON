final class Locks {
    private static let api = "locks"

    func request(_ name: String) throws {
        _ = try TacWebHost.call(Self.api, "request", [name])
    }

    func release(_ name: String) throws {
        _ = try TacWebHost.call(Self.api, "release", [name])
    }
}
