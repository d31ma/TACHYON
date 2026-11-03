final class Share {
    func open(title: String, text: String, url: String) throws {
        _ = try TacWebHost.call("share", "open", [title, text, url])
    }
}
