final class WindowControls {
    func granted(_ control: String) throws -> Bool {
        try TacWebHost.answer(TacWebHost.call("windowControls", "granted", [control]), as: Bool.self)
    }

    func minimize() throws {
        _ = try TacWebHost.call("windowControls", "minimize")
    }

    func maximize() throws {
        _ = try TacWebHost.call("windowControls", "maximize")
    }

    func fullscreen() throws {
        _ = try TacWebHost.call("windowControls", "fullscreen")
    }

    func close() throws {
        _ = try TacWebHost.call("windowControls", "close")
    }

    func resize(width: Int, height: Int) throws {
        _ = try TacWebHost.call("windowControls", "resize", [width, height])
    }

    func setTitle(_ text: String) throws {
        _ = try TacWebHost.call("windowControls", "setTitle", [text])
    }
}
