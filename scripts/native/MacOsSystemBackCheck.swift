import Darwin

@MainActor
private final class MacOsSystemBackPages: NSObject, WKURLSchemeHandler {
    func webView(_ webView: WKWebView, start task: any WKURLSchemeTask) {
        let path = task.request.url?.path ?? "/"
        let body = Data("<!doctype html><title>\(path)</title><h1>\(path)</h1>".utf8)
        task.didReceive(URLResponse(url: task.request.url!, mimeType: "text/html", expectedContentLength: body.count,
                                    textEncodingName: "utf-8"))
        task.didReceive(body)
        task.didFinish()
    }

    func webView(_ webView: WKWebView, stop task: any WKURLSchemeTask) {}
}

@main
@MainActor
private struct MacOsSystemBackCheck {
    private static var result: Result<Void, Error>?

    static func main() throws {
        let application = NSApplication.shared
        _ = application.setActivationPolicy(.accessory)
        guard let wake = NSEvent.otherEvent(with: .applicationDefined, location: .zero, modifierFlags: [],
                                            timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: 0,
                                            context: nil, subtype: 0, data1: 0, data2: 0) else { throw failure("No wake event") }
        let watchdog = DispatchSource.makeTimerSource(queue: DispatchQueue.global(qos: .utility))
        watchdog.schedule(deadline: .now() + 60)
        watchdog.setEventHandler { () -> Void in
            FileHandle.standardError.write(Data("FAIL: the system back check exceeded 60 seconds\n".utf8))
            Darwin.exit(1)
        }
        watchdog.resume()
        defer { watchdog.cancel() }
        Timer.scheduledTimer(withTimeInterval: 0, repeats: false) { (_: Timer) -> Void in
            MainActor.assumeIsolated { () -> Void in
                result = Result { try exercise(application) }
                application.stop(nil)
                application.postEvent(wake, atStart: true)
            }
        }
        application.run()
        guard let result else { throw failure("The application stopped before the checks ran") }
        try result.get()
        FileHandle.standardOutput.write(Data("PASS native macOS View menu Back and Forward, Command-[ and Command-], through the web view history\n".utf8))
    }

    private static func exercise(_ application: NSApplication) throws {
        let configuration = WKWebViewConfiguration()
        let pages = MacOsSystemBackPages()
        configuration.setURLSchemeHandler(pages, forURLScheme: "back-check")
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 480, height: 320),
                              styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        let view = WKWebView(frame: window.contentView!.bounds, configuration: configuration)
        window.contentView = view
        window.makeKeyAndOrderFront(nil)
        let menus = MacOsMenus()
        menus.install(application, window, "Back Check", nil, nil, MacOsMenuHistory(view, window))
        guard let menu = application.mainMenu?.items.first(where: { (item: NSMenuItem) -> Bool in item.title == "View" })?.submenu,
              let back = menu.item(at: 0), let forward = menu.item(at: 1) else { throw failure("View menu has no Back and Forward") }
        try require(back.title == "Back" && back.keyEquivalent == "[" && back.keyEquivalentModifierMask == .command, "Back is not Command-[")
        try require(forward.title == "Forward" && forward.keyEquivalent == "]" && forward.keyEquivalentModifierMask == .command, "Forward is not Command-]")
        try require(menu.item(at: 3)?.action == #selector(NSWindow.toggleFullScreen(_:)), "Enter Full Screen moved")
        try open(view, "/")
        try require(!enabled(menu, back) && !enabled(menu, forward), "Back or Forward is enabled on the first page")
        try open(view, "/second/")
        try require(enabled(menu, back) && !enabled(menu, forward), "Back is not the only enabled item on the second page")
        try require(press(application, window, "[", 33), "Command-[ was not a menu key equivalent")
        try settle(view, "/")
        try require(enabled(menu, forward), "Forward is disabled after going back")
        try require(press(application, window, "]", 30), "Command-] was not a menu key equivalent")
        try settle(view, "/second/")
        try require(enabled(menu, back) && !enabled(menu, forward), "Back is not the only enabled item after going forward")
        menu.performActionForItem(at: 0)
        try settle(view, "/")
        window.close()
        try require(!enabled(menu, back) && !enabled(menu, forward), "Back or Forward stayed enabled after the window closed")
        withExtendedLifetime((menus, pages)) {}
    }

    private static func open(_ view: WKWebView, _ path: String) throws {
        view.load(URLRequest(url: URL(string: "back-check://app" + path)!))
        try settle(view, path)
    }

    private static func settle(_ view: WKWebView, _ path: String) throws {
        let deadline = Date().addingTimeInterval(15)
        while Date() < deadline {
            if view.url?.path == path || (path.hasSuffix("/") && view.url?.path == String(path.dropLast())), !view.isLoading { return }
            RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.05))
        }
        throw failure("The web view did not reach \(path); it shows \(view.url?.absoluteString ?? "nothing")")
    }

    private static func enabled(_ menu: NSMenu, _ item: NSMenuItem) -> Bool {
        menu.update()
        return item.isEnabled
    }

    private static func press(_ application: NSApplication, _ window: NSWindow, _ key: String, _ code: UInt16) -> Bool {
        guard let event = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: .command,
                                           timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                                           context: nil, characters: key, charactersIgnoringModifiers: key,
                                           isARepeat: false, keyCode: code) else { return false }
        return application.mainMenu?.performKeyEquivalent(with: event) ?? false
    }

    private static func require(_ condition: Bool, _ message: String) throws {
        guard condition else { throw failure(message) }
    }

    private static func failure(_ message: String) -> NSError {
        FileHandle.standardError.write(Data("FAIL: \(message)\n".utf8))
        return NSError(domain: message, code: 1)
    }
}
