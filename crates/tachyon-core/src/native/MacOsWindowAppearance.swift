@MainActor
private enum MacOsWindowAppearance {
    static let themeColor: NSColor? = __WINDOW_THEME_COLOR__
    static let backgroundColor: NSColor? = __WINDOW_BACKGROUND_COLOR__
    static let titleAppearance: NSAppearance.Name? = __WINDOW_TITLE_APPEARANCE__
    static let fullscreen = __WINDOW_FULLSCREEN__
    static var pageAppearance: NSKeyValueObservation?

    static func prepare(_ window: NSWindow, _ view: MacOsTitleBarWebView) {
        view.backdrop = backgroundColor ?? themeColor.map { (_: NSColor) -> NSColor in NSColor.windowBackgroundColor }
        if let backgroundColor { window.backgroundColor = backgroundColor }
        guard let themeColor else { return }
        window.titlebarAppearsTransparent = true
        window.backgroundColor = themeColor
        window.appearance = titleAppearance.flatMap { (name: NSAppearance.Name) -> NSAppearance? in NSAppearance(named: name) }
        pageAppearance = NSApp.observe(\.effectiveAppearance, options: [.initial, .new]) { [weak view] (application: NSApplication, _: NSKeyValueObservedChange<NSAppearance>) -> Void in
            MainActor.assumeIsolated { view?.appearance = application.effectiveAppearance }
        }
    }

    static func open(_ window: NSWindow) {
        if fullscreen { window.toggleFullScreen(nil) }
    }
}
