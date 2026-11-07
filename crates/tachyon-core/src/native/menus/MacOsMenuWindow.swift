@MainActor
final class MacOsMenuWindow: NSObject {
    private weak var window: NSWindow?
    private var closed = false

    init(_ window: NSWindow) {
        self.window = window
        super.init()
        NotificationCenter.default.addObserver(self, selector: #selector(willClose(_:)),
                                               name: NSWindow.willCloseNotification, object: window)
    }

    deinit {
        NotificationCenter.default.removeObserver(self)
    }

    func owns(_ view: WKWebView) -> Bool {
        attached(view)
    }

    private func attached(_ view: WKWebView) -> Bool {
        guard !closed, let window else { return false }
        return view.window === window && window.contentView === view
    }

    @objc private func willClose(_ notification: Notification) {
        closed = true
    }
}
