@MainActor
final class MacOsMenuActions: NSObject, NSMenuItemValidation {
    private weak var view: WKWebView?
    private let window: MacOsMenuWindow?
    private let actions: [MacOsMenuAction]
    private let ready: (String) -> Bool
    private let perform: (String) -> Bool

    init(_ view: WKWebView?, _ window: NSWindow?, _ actions: [MacOsMenuAction],
         _ ready: @escaping (String) -> Bool, _ perform: @escaping (String) -> Bool) {
        self.view = view
        self.window = window.map { (owned: NSWindow) -> MacOsMenuWindow in MacOsMenuWindow(owned) }
        self.actions = actions
        self.ready = ready
        self.perform = perform
        super.init()
    }

    func install(_ fileMenu: NSMenu, _ viewMenu: NSMenu, _ helpMenu: NSMenu) {
        append(.file, fileMenu)
        append(.view, viewMenu)
        append(.help, helpMenu)
    }

    func validateMenuItem(_ item: NSMenuItem) -> Bool {
        enabled(item) != nil
    }

    @objc private func run(_ item: NSMenuItem) {
        guard let action = enabled(item), perform(action.id) else {
            NSSound.beep()
            return
        }
    }

    private func enabled(_ item: NSMenuItem) -> MacOsMenuAction? {
        guard actions.count <= 64, let entry = item.representedObject as? MacOsMenuAction,
              let declared = actions.first(where: { (action: MacOsMenuAction) -> Bool in action.id == entry.id }) else { return nil }
        return declared.event.isEmpty || pageListens(declared.event) ? declared : nil
    }

    private func pageListens(_ event: String) -> Bool {
        guard let view, let window, window.owns(view) else { return false }
        return ready(event)
    }

    private func append(_ group: MacOsMenuGroup, _ menu: NSMenu) {
        actions.filter { (action: MacOsMenuAction) -> Bool in action.group == group }
            .forEach { (action: MacOsMenuAction) -> Void in menu.addItem(item(action)) }
    }

    private func item(_ action: MacOsMenuAction) -> NSMenuItem {
        let item = NSMenuItem(title: action.label, action: #selector(run(_:)), keyEquivalent: action.shortcut.key)
        item.identifier = NSUserInterfaceItemIdentifier(action.id)
        item.keyEquivalentModifierMask = action.shortcut.modifiers
        item.target = self
        item.representedObject = action
        return item
    }
}
