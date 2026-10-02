@main
private struct MacOsMenuCheck {
    @MainActor
    static func main() throws {
        let application = NSApplication.shared
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 320, height: 240),
                              styleMask: [.titled, .closable, .miniaturizable, .resizable],
                              backing: .buffered, defer: false)
        MacOsMenus().install(application, window, "Menu Check")
        try checkApplication(application)
        try checkWindow(application, window)
        try checkEdit(application, window)
        try checkAbsentDocuments(application)
        window.close()
        print("PASS native macOS menu roles, actions, ownership and text responder")
    }

    @MainActor
    private static func menu(_ application: NSApplication, _ title: String) throws -> NSMenu {
        guard let menu = application.mainMenu?.items.first(where: { (item: NSMenuItem) -> Bool in item.title == title })?.submenu else {
            throw NSError(domain: "Missing native menu group \(title)", code: 1)
        }
        return menu
    }

    @MainActor
    private static func checkApplication(_ application: NSApplication) throws {
        let app = try menu(application, "Menu Check")
        try require(app.title == "Menu Check", "Application identity changed")
        try require(app.item(at: 0)?.action == #selector(NSApplication.orderFrontStandardAboutPanel(_:)), "About is not standard AppKit")
        try require(app.item(at: 2)?.action == #selector(NSApplication.hide(_:)), "Hide action missing")
        try require(app.item(at: 3)?.keyEquivalentModifierMask == [.command, .option], "Hide Others shortcut changed")
        try require(app.item(at: 4)?.action == #selector(NSApplication.unhideAllApplications(_:)), "Show All missing")
        try require(app.item(at: 6)?.action == #selector(NSApplication.terminate(_:)), "Quit bypasses NSApplication termination")
        try require(app.item(at: 6)?.target as? NSApplication === application, "Quit targets another owner")
        try require(app.item(at: 6)?.keyEquivalent == "q", "Quit shortcut missing")
    }

    @MainActor
    private static func checkWindow(_ application: NSApplication, _ window: NSWindow) throws {
        let view = try menu(application, "View")
        let windows = try menu(application, "Window")
        let help = try menu(application, "Help")
        try require(view.item(at: 0)?.target as? NSWindow === window, "View targets another window")
        try require(view.item(at: 0)?.action == #selector(NSWindow.toggleFullScreen(_:)), "Full screen action missing")
        try require(view.item(at: 0)?.keyEquivalentModifierMask == [.command, .control], "Full screen shortcut changed")
        try require(application.windowsMenu === windows, "Window menu role not registered")
        try require(windows.item(at: 0)?.target as? NSWindow === window, "Minimize targets another window")
        try require(windows.item(at: 0)?.keyEquivalent == "m", "Minimize shortcut missing")
        try require(windows.item(at: 1)?.action == #selector(NSWindow.performZoom(_:)), "Zoom missing")
        try require(application.helpMenu === help, "Help search role not registered")
    }

    @MainActor
    private static func checkEdit(_ application: NSApplication, _ window: NSWindow) throws {
        let edit = try menu(application, "Edit")
        try require(edit.items.filter { (item: NSMenuItem) -> Bool in !item.isSeparatorItem }
                    .allSatisfy { (item: NSMenuItem) -> Bool in item.target == nil }, "Edit captures a component instead of the responder chain")
        try require(edit.item(at: 0)?.keyEquivalent == "z", "Undo shortcut missing")
        try require(edit.item(at: 1)?.keyEquivalentModifierMask == [.command, .shift], "Redo shortcut changed")
        let text = NSTextView(frame: window.contentView!.bounds)
        text.string = "native responder text"
        window.contentView = text
        window.makeKeyAndOrderFront(nil)
        try require(window.makeFirstResponder(text), "Text responder not installed")
        let select = edit.item(at: 6)!
        try require(select.action == #selector(NSText.selectAll(_:)), "Select All action missing")
        try require(application.sendAction(select.action!, to: select.target, from: select), "Native responder declined Select All")
        try require(text.selectedRange() == NSRange(location: 0, length: text.string.utf16.count), "Select All did not reach native text responder")
    }

    @MainActor
    private static func checkAbsentDocuments(_ application: NSApplication) throws {
        let groups = application.mainMenu!.items.compactMap { (item: NSMenuItem) -> NSMenu? in item.submenu }
        let actions = groups.flatMap { (menu: NSMenu) -> [NSMenuItem] in menu.items }
            .compactMap { (item: NSMenuItem) -> Selector? in item.action }
            .map { (action: Selector) -> String in NSStringFromSelector(action) }
        try require(!actions.contains("newDocument:"), "Inapplicable New document action exists")
        try require(!actions.contains("openDocument:"), "Inapplicable Open document action exists")
        try require(!actions.contains("saveDocument:"), "Inapplicable Save document action exists")
        try require(!actions.contains("print:"), "Inapplicable Print action exists")
    }

    private static func require(_ condition: Bool, _ message: String) throws {
        guard condition else { throw NSError(domain: message, code: 1) }
    }
}
