@MainActor
final class MacOsTitleBar: NSObject {
    static let pageScript = #"""
__NATIVE_TITLE_BAR_PAGE_SCRIPT__
"""#
    static let windowChanges: [Notification.Name] = [
        NSWindow.didResizeNotification,
        NSWindow.willEnterFullScreenNotification,
        NSWindow.didEnterFullScreenNotification,
        NSWindow.willExitFullScreenNotification,
        NSWindow.didExitFullScreenNotification,
        NSWindow.didChangeBackingPropertiesNotification,
        NSWindow.didChangeScreenNotification,
        NSWindow.didBecomeKeyNotification,
        NSWindow.didResignKeyNotification,
    ]

    private let settings: MacOsTitleBarSettings
    private weak var window: NSWindow?
    private weak var view: WKWebView?
    private var baseScripts: [WKUserScript] = []
    private var regions = MacOsDragRegions.empty
    private var published: MacOsTitleBarInsets?
    private var appearance: NSKeyValueObservation?

    init(_ settings: MacOsTitleBarSettings) {
        self.settings = settings
        super.init()
    }

    deinit {
        NotificationCenter.default.removeObserver(self)
    }

    var styleMask: NSWindow.StyleMask {
        var mask: NSWindow.StyleMask = [.titled, .closable, .miniaturizable, .resizable]
        if settings.integrated { mask.insert(.fullSizeContentView) }
        return mask
    }

    func install(_ window: NSWindow, _ view: MacOsTitleBarWebView) {
        guard settings.integrated else { return }
        self.window = window
        self.view = view
        baseScripts = view.configuration.userContentController.userScripts
        window.titlebarAppearsTransparent = true
        window.titleVisibility = .hidden
        window.titlebarSeparatorStyle = .none
        view.titleBar = self
        for name in MacOsTitleBar.windowChanges {
            NotificationCenter.default.addObserver(self, selector: #selector(windowChanged(_:)), name: name, object: window)
        }
        appearance = window.observe(\.effectiveAppearance) { [weak self] (_: NSWindow, _: NSKeyValueObservedChange<NSAppearance>) -> Void in
            MainActor.assumeIsolated { self?.layOut() }
        }
        layOut()
    }

    func accessibleTitle(_ requested: String) -> String {
        settings.integrated && requested.isEmpty ? settings.applicationName : requested
    }

    func updateRegions(_ payload: String) -> String {
        guard let request = tachyonParseJSONRequest(payload), let parsed = MacOsDragRegions.parse(request) else {
            return "{\"error\":\"Drag regions must be at most 256 rectangles of four numbers.\"}"
        }
        regions = parsed
        return "{\"ok\":true}"
    }

    func contains(_ event: NSEvent, _ view: WKWebView) -> Bool {
        regions.contains(pagePoint(event, view))
    }

    func drag(_ event: NSEvent, _ view: WKWebView) -> Bool {
        guard let window, contains(event, view) else { return false }
        if event.clickCount > 1 {
            doubleClick(event, window)
        } else {
            window.performDrag(with: event)
        }
        return true
    }

    func insets() -> MacOsTitleBarInsets {
        guard let window, !window.styleMask.contains(.fullScreen) else { return MacOsTitleBarInsets.none }
        let buttons = MacOsWindowButtons.extent(window)
        let bar = window.frame.height - window.contentLayoutRect.maxY
        guard !buttons.isNull else { return MacOsTitleBarInsets(height: Int(bar.rounded(.up)), leading: 0, trailing: 0) }
        let buttonTop = window.frame.height - buttons.maxY
        let height = max(bar, buttons.height + buttonTop * 2)
        let leftToRight = buttons.midX < window.frame.width / 2
        let leading = leftToRight ? buttons.maxX + buttons.minX : (window.frame.width - buttons.minX) * 2 - buttons.width
        return MacOsTitleBarInsets(height: Int(height.rounded(.up)), leading: Int(leading.rounded(.up)), trailing: 0)
    }

    private func pagePoint(_ event: NSEvent, _ view: WKWebView) -> NSPoint {
        let local = view.convert(event.locationInWindow, from: nil)
        let top = view.isFlipped ? local.y : view.bounds.height - local.y
        let zoom = max(view.pageZoom, 0.1)
        return NSPoint(x: local.x / zoom, y: top / zoom)
    }

    private func doubleClick(_ event: NSEvent, _ window: NSWindow) {
        guard let frameView = window.contentView?.superview else { return }
        let strip = max(window.frame.height - window.contentLayoutRect.maxY, 2)
        let x = max(event.locationInWindow.x, CGFloat(insets().leading))
        let location = NSPoint(x: x, y: window.frame.height - strip / 2)
        let relocated = NSEvent.mouseEvent(
            with: .leftMouseDown, location: location, modifierFlags: event.modifierFlags,
            timestamp: event.timestamp, windowNumber: event.windowNumber, context: nil,
            eventNumber: event.eventNumber, clickCount: event.clickCount, pressure: event.pressure)
        guard let relocated else { return }
        frameView.mouseDown(with: relocated)
    }

    @objc private func windowChanged(_ notification: Notification) {
        layOut()
    }

    private func layOut() {
        guard let window else { return }
        if let inset = settings.buttonInset { MacOsWindowButtons.place(window, inset) }
        publish()
    }

    private func publish() {
        let current = insets()
        guard current != published, let view else { return }
        published = current
        let controller = view.configuration.userContentController
        controller.removeAllUserScripts()
        for script in baseScripts { controller.addUserScript(script) }
        let source = MacOsTitleBar.pageScript + "\n" + current.pageCall
        controller.addUserScript(WKUserScript(source: source, injectionTime: .atDocumentStart, forMainFrameOnly: true))
        view.evaluateJavaScript(current.pageCall)
    }
}

