final class Preferences {
    func scheme() throws -> String {
        try text("scheme")
    }

    func motion() throws -> String {
        try text("motion")
    }

    func language() throws -> String {
        try text("language")
    }

    private func text(_ call: String) throws -> String {
        try TacWebHost.text("preferences", call)
    }
}
