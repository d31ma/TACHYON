import Darwin

@main
@MainActor
private struct MacOsMenuCheck {
    private static var result: Result<Void, Error>?

    static func main() throws {
        let application = NSApplication.shared
        try require(application.setActivationPolicy(.regular), "Test application could not use regular activation policy")
        let wake = try wakeEvent()
        let watchdog = startWatchdog()
        defer { watchdog.cancel() }
        progress("AppKit loop starting")
        DispatchQueue.main.async { @MainActor () -> Void in
            progress("AppKit main queue entered")
            do {
                try exercise(application)
                result = .success(())
            } catch {
                progress("FAIL: \(error)")
                result = .failure(error)
            }
            progress("AppKit stop requested")
            application.stop(nil)
            application.postEvent(wake, atStart: true)
        }
        application.run()
        progress("AppKit loop returned")
        guard let result else { throw NSError(domain: "Menu application stopped before its checks completed", code: 1) }
        try result.get()
        FileHandle.standardOutput.write(Data("PASS native macOS menu roles, actions, ownership and text responder\n".utf8))
    }

    private static func wakeEvent() throws -> NSEvent {
        guard let event = NSEvent.otherEvent(with: .applicationDefined, location: .zero, modifierFlags: [],
                                            timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: 0,
                                            context: nil, subtype: 0, data1: 0, data2: 0) else {
            throw NSError(domain: "Test AppKit wake event could not be created", code: 1)
        }
        return event
    }

    private static func startWatchdog() -> DispatchSourceTimer {
        let timer = DispatchSource.makeTimerSource(queue: DispatchQueue.global(qos: .utility))
        timer.schedule(deadline: .now() + 10)
        timer.setEventHandler { () -> Void in
            FileHandle.standardError.write(Data("FAIL: owned menu fixture exceeded its 10-second runtime deadline\n".utf8))
            Darwin.exit(1)
        }
        timer.resume()
        return timer
    }

    private static func progress(_ message: String) {
        FileHandle.standardError.write(Data((message + "\n").utf8))
    }

    private static func exercise(_ application: NSApplication) throws {
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 320, height: 240),
                              styleMask: [.titled, .closable, .miniaturizable, .resizable],
                              backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        defer { window.close() }
        MacOsMenus().install(application, window, "Menu Check")
        try checkApplication(application)
        try checkWindow(application, window)
        progress("Application and Window menu checks passed")
        try checkEdit(application, window)
        progress("Nil-target native Select All and selection checks passed")
        try checkAbsentDocuments(application)
        try MacOsMenuOwnershipCheck().run(application, window)
        progress("All menu and ownership checks completed")
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
        guard let fullscreen = view.item(at: 0) else { throw NSError(domain: "Full screen item missing", code: 1) }
        let shortcut = shortcutState(fullscreen)
        progress("Full-screen shortcut: " + shortcut)
        try require(fullscreen.target as? NSWindow === window, "View targets another window: " + shortcut)
        try require(fullscreen.action == #selector(NSWindow.toggleFullScreen(_:)), "Full screen action missing: " + shortcut)
        try require(fullscreen.keyEquivalentModifierMask == [.command, .control], "Full screen shortcut changed: " + shortcut)
        try require(application.windowsMenu === windows, "Window menu role not registered")
        try require(windows.item(at: 0)?.target as? NSWindow === window, "Minimize targets another window")
        try require(windows.item(at: 0)?.keyEquivalent == "m", "Minimize shortcut missing")
        try require(windows.item(at: 1)?.action == #selector(NSWindow.performZoom(_:)), "Zoom missing")
        try require(application.helpMenu === help, "Help search role not registered")
    }

    private static func shortcutState(_ item: NSMenuItem) -> String {
        "title=\(item.title.debugDescription), key=\(item.keyEquivalent.debugDescription), modifiers=0x\(String(item.keyEquivalentModifierMask.rawValue, radix: 16)), userKey=\(item.userKeyEquivalent.debugDescription), userEquivalents=\(NSMenuItem.usesUserKeyEquivalents)"
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
        application.activate(ignoringOtherApps: true)
        progress("Waiting for owned native text responder")
        try waitForResponder(application, window, text)
        progress("Owned responder ready: " + responderState(application, window, text))
        let select = edit.item(at: 6)!
        try require(select.action == #selector(NSText.selectAll(_:)), "Select All action missing")
        try require(application.sendAction(select.action!, to: select.target, from: select), "Native responder declined Select All: " + responderState(application, window, text))
        try require(text.selectedRange() == NSRange(location: 0, length: text.string.utf16.count), "Select All did not reach native text responder: " + responderState(application, window, text))
    }

    private static func waitForResponder(_ application: NSApplication, _ window: NSWindow, _ text: NSTextView) throws {
        let deadline = ProcessInfo.processInfo.systemUptime + 3
        while !responderReady(application, window, text) {
            guard ProcessInfo.processInfo.systemUptime < deadline else {
                throw NSError(domain: "Native responder readiness timed out: " + responderState(application, window, text), code: 1)
            }
            if let event = application.nextEvent(matching: .any, until: Date(timeIntervalSinceNow: 0.01), inMode: .default, dequeue: true) {
                application.sendEvent(event)
            }
        }
    }

    private static func responderReady(_ application: NSApplication, _ window: NSWindow, _ text: NSTextView) -> Bool {
        application.isActive && application.keyWindow === window && window.isKeyWindow && window.firstResponder === text
    }

    private static func responderState(_ application: NSApplication, _ window: NSWindow, _ text: NSTextView) -> String {
        let target: Any? = application.target(forAction: #selector(NSText.selectAll(_:)))
        return "active=\(application.isActive), key=\(application.keyWindow === window), windowKey=\(window.isKeyWindow), firstResponder=\(window.firstResponder === text), selectAllTarget=\(target as? NSTextView === text), visible=\(window.isVisible), selection=\(NSStringFromRange(text.selectedRange())), textLength=\(text.string.utf16.count)"
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
