final class Clipboard {
    func writeText(_ text: String) throws {
        _ = try TacWebHost.call("clipboard", "writeText", [text])
    }

    func readText() throws {
        _ = try TacWebHost.call("clipboard", "readText")
    }
}
