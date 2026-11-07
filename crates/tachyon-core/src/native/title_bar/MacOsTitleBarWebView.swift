@MainActor
final class MacOsTitleBarWebView: WKWebView {
    weak var titleBar: MacOsTitleBar?
    var backdrop: NSColor? {
        didSet { paintBackdrop() }
    }

    override func mouseDown(with event: NSEvent) {
        if titleBar?.drag(event, self) == true { return }
        super.mouseDown(with: event)
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        if let event, titleBar?.contains(event, self) == true { return true }
        return super.acceptsFirstMouse(for: event)
    }

    override func updateLayer() {
        super.updateLayer()
        paintBackdrop()
    }

    override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        paintBackdrop()
    }

    private func paintBackdrop() {
        guard let backdrop else { return }
        wantsLayer = true
        effectiveAppearance.performAsCurrentDrawingAppearance { () -> Void in
            layer?.backgroundColor = backdrop.cgColor
        }
    }
}

