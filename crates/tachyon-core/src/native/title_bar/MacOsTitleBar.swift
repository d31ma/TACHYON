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

    static let doubleClickSetting = "AppleActionOnDoubleClick"
    static let legacyMinimizeSetting = "AppleMiniaturizeOnDoubleClick"
    static let minimizeAction = "Minimize"
    static let zoomAction = "Maximize"
    static let noActions = ["None", "Do Nothing"]

    private let settings: MacOsTitleBarSettings
    private weak var window: NSWindow?
    private weak var view: WKWebView?
    private var baseScripts: [WKUserScript] = []
    private var regions = MacOsDragRegions.empty
    private var published: MacOsTitleBarInsets?
    private var appearance: NSKeyValueObservation?
    private var watchedViews: [ObjectIdentifier] = []
    private var placing = false
    private var buttonSpacing: CGFloat = 0

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
        baseScripts = view.configuration.userContentController.userScripts.map { (script: WKUserScript) -> WKUserScript in script }
        window.titlebarAppearsTransparent = true
        window.titleVisibility = .hidden
        window.titlebarSeparatorStyle = .none
        view.titleBar = self
        buttonSpacing = MacOsWindowButtons.spacing(window)
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
            doubleClick(window)
        } else {
            window.performDrag(with: event)
        }
        return true
    }

    func insets() -> MacOsTitleBarInsets {
        guard let window, !window.styleMask.contains(.fullScreen) else { return MacOsTitleBarInsets.zero }
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

    private func doubleClick(_ window: NSWindow) {
        let defaults = UserDefaults.standard
        let minimizes = defaults.bool(forKey: MacOsTitleBar.legacyMinimizeSetting)
        let fallback = minimizes ? MacOsTitleBar.minimizeAction : MacOsTitleBar.zoomAction
        let action = defaults.string(forKey: MacOsTitleBar.doubleClickSetting) ?? fallback
        if action == MacOsTitleBar.minimizeAction {
            window.miniaturize(nil)
        } else if !MacOsTitleBar.noActions.contains(action) {
            window.zoom(nil)
        }
    }

    @objc private func windowChanged(_ notification: Notification) {
        layOut()
    }

    private func layOut() {
        guard let window, !placing else { return }
        placing = true
        if let inset = settings.buttonInset {
            MacOsWindowButtons.place(window, inset, buttonSpacing)
            watchLayout(window)
        }
        placing = false
        publish()
    }

    private func watchLayout(_ window: NSWindow) {
        for view in MacOsWindowButtons.layoutViews(window) {
            let identifier = ObjectIdentifier(view)
            guard !watchedViews.contains(identifier) else { continue }
            watchedViews.append(identifier)
            view.postsFrameChangedNotifications = true
            NotificationCenter.default.addObserver(self, selector: #selector(windowChanged(_:)),
                                                   name: NSView.frameDidChangeNotification, object: view)
        }
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

