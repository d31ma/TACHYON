final class PageVisibility {
    func visible() throws -> Bool {
        try TacWebHost.flag("pageVisibility", "visible")
    }
}
