@MainActor
final class MacOsMenuNavigation: NSObject, NSMenuItemValidation {
    private weak var view: WKWebView?
    private let window: MacOsMenuWindow
    private let documents: [String: String]
    private let routes: [MacOsMenuRoute]

    init(_ view: WKWebView, _ window: NSWindow, _ documents: [String: String], _ routes: [MacOsMenuRoute]) {
        self.view = view
        self.window = MacOsMenuWindow(window)
        self.documents = documents
        self.routes = routes
        super.init()
    }

    func install(_ root: NSMenu, _ viewMenu: NSMenu, _ helpMenu: NSMenu) {
        _ = fileGroup().map { (group: NSMenuItem) -> Void in root.insertItem(group, at: 1) }
        append(.view, viewMenu)
        append(.help, helpMenu)
    }

    func validateMenuItem(_ item: NSMenuItem) -> Bool {
        destination(item) != nil
    }

    @objc private func navigate(_ item: NSMenuItem) {
        guard let url = destination(item) else { return }
        view?.load(URLRequest(url: url))
    }

    private func destination(_ item: NSMenuItem) -> URL? {
        guard let view, window.owns(view), routes.count <= 64,
              let entry = item.representedObject as? MacOsMenuRoute,
              routes.contains(where: { (route: MacOsMenuRoute) -> Bool in route.id == entry.id && route.destination.route == entry.destination.route }),
              documents[entry.destination.route] != nil else { return nil }
        return URL(string: "tachyon-app://bundle" + entry.destination.route)
    }

    private func fileGroup() -> NSMenuItem? {
        let menu = NSMenu(title: "File")
        append(.file, menu)
        guard menu.numberOfItems > 0 else { return nil }
        let item = NSMenuItem(title: "File", action: nil, keyEquivalent: "")
        item.submenu = menu
        return item
    }

    private func append(_ group: MacOsMenuGroup, _ menu: NSMenu) {
        routes.filter { (route: MacOsMenuRoute) -> Bool in route.destination.group == group }
            .forEach { (route: MacOsMenuRoute) -> Void in menu.addItem(item(route)) }
    }

    private func item(_ route: MacOsMenuRoute) -> NSMenuItem {
        let item = NSMenuItem(title: route.label, action: #selector(navigate(_:)), keyEquivalent: route.shortcut.key)
        item.identifier = NSUserInterfaceItemIdentifier(route.id)
        item.keyEquivalentModifierMask = route.shortcut.modifiers
        item.target = self
        item.representedObject = route
        return item
    }
}
