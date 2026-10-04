@MainActor
final class MacOsTitleBarWebView: WKWebView {
    weak var titleBar: MacOsTitleBar?

    override func mouseDown(with event: NSEvent) {
        if titleBar?.drag(event, self) == true { return }
        super.mouseDown(with: event)
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        if let event, titleBar?.contains(event, self) == true { return true }
        return super.acceptsFirstMouse(for: event)
    }
}

