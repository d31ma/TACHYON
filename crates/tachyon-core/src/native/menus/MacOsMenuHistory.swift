@MainActor
final class MacOsMenuHistory: NSObject, NSMenuItemValidation {
    private weak var view: WKWebView?
    private let window: MacOsMenuWindow?

    init(_ view: WKWebView?, _ window: NSWindow?) {
        self.view = view
        self.window = window.map { (owned: NSWindow) -> MacOsMenuWindow in MacOsMenuWindow(owned) }
        super.init()
    }

    func install(_ menu: NSMenu) {
        menu.addItem(item("Back", #selector(back(_:)), "["))
        menu.addItem(item("Forward", #selector(forward(_:)), "]"))
        menu.addItem(.separator())
    }

    func validateMenuItem(_ item: NSMenuItem) -> Bool {
        guard let view = shown() else { return false }
        return item.action == #selector(back(_:)) ? view.canGoBack : view.canGoForward
    }

    @objc private func back(_ item: NSMenuItem) {
        guard let view = shown(), view.canGoBack else { return }
        Telemetry.shared.record("navigation.back")
        view.goBack()
    }

    @objc private func forward(_ item: NSMenuItem) {
        guard let view = shown(), view.canGoForward else { return }
        Telemetry.shared.record("navigation.forward")
        view.goForward()
    }

    private func shown() -> WKWebView? {
        guard let view, let window, window.owns(view) else { return nil }
        return view
    }

    private func item(_ title: String, _ action: Selector, _ key: String) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: key)
        item.keyEquivalentModifierMask = .command
        item.target = self
        return item
    }
}
