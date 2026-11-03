@MainActor
final class MacOsMenuOwnershipCheck {
    func run(_ application: NSApplication, _ window: NSWindow) throws {
        let view = WKWebView(frame: window.contentView!.bounds)
        window.contentView = view
        window.isReleasedWhenClosed = false
        let route = entry("/status")
        let owner = MacOsMenuNavigation(view, window, ["/status":"status/index.html"], [route])
        MacOsMenus().install(application, window, "Menu Check", owner)
        let item = try selected(application)
        try require(owner.validateMenuItem(item), "Attached owned view was refused")
        try checkHidden(owner, item, window)
        try checkUnregistered(owner, item)
        try checkBound(view, window, item, route)
        try checkDetached(owner, item, view, window)
        try checkClosed(owner, item, window)
    }

    private func entry(_ route: String) -> MacOsMenuRoute {
        MacOsMenuRoute(id: "status", label: "Status", destination: MacOsMenuDestination(group: .file, route: route),
                       shortcut: MacOsMenuShortcut(key: "1", modifiers: .command))
    }

    private func selected(_ application: NSApplication) throws -> NSMenuItem {
        guard let file = application.mainMenu?.items.first(where: { (item: NSMenuItem) -> Bool in item.title == "File" })?.submenu,
              let item = file.item(at: 0) else { throw NSError(domain: "Declared native File menu missing", code: 1) }
        return item
    }

    private func checkHidden(_ owner: MacOsMenuNavigation, _ item: NSMenuItem, _ window: NSWindow) throws {
        window.orderOut(nil)
        try require(!window.isVisible, "Owned window did not hide")
        try require(owner.validateMenuItem(item), "Hidden attached live window was incorrectly retired")
    }

    private func checkUnregistered(_ owner: MacOsMenuNavigation, _ item: NSMenuItem) throws {
        let original = item.representedObject
        item.representedObject = entry("/missing")
        try require(!owner.validateMenuItem(item), "Unregistered menu route was accepted")
        item.representedObject = original
    }

    private func checkBound(_ view: WKWebView, _ window: NSWindow, _ item: NSMenuItem, _ route: MacOsMenuRoute) throws {
        let oversized = MacOsMenuNavigation(view, window, ["/status":"status/index.html"], Array(repeating: route, count: 65))
        try require(!oversized.validateMenuItem(item), "Oversized route capture remained active")
    }

    private func checkDetached(_ owner: MacOsMenuNavigation, _ item: NSMenuItem, _ view: WKWebView, _ window: NSWindow) throws {
        window.contentView = NSView()
        try require(!owner.validateMenuItem(item), "Replaced owned view retained native menu authority")
        view.removeFromSuperview()
        try require(!owner.validateMenuItem(item), "Unattached view retained native menu authority")
        window.contentView = view
        try require(owner.validateMenuItem(item), "Same attached live owner was refused")
    }

    private func checkClosed(_ owner: MacOsMenuNavigation, _ item: NSMenuItem, _ window: NSWindow) throws {
        window.close()
        try require(!owner.validateMenuItem(item), "Closed window retained native menu authority")
        window.makeKeyAndOrderFront(nil)
        try require(!owner.validateMenuItem(item), "Reopening a closed window revived the retired menu target")
        window.close()
    }

    private func require(_ condition: Bool, _ message: String) throws {
        guard condition else { throw NSError(domain: message, code: 1) }
    }
}
