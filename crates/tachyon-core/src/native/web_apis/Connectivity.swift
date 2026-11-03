final class Connectivity {
    func online() throws -> Bool {
        try TacWebHost.flag("connectivity", "online")
    }
}
