final class Focus {
    func move(_ target: String) throws {
        _ = try TacWebHost.call("focus", "move", [target])
    }

    func trap(_ container: String) throws {
        _ = try TacWebHost.call("focus", "trap", [container])
    }

    func release() throws {
        _ = try TacWebHost.call("focus", "release", [])
    }
}
