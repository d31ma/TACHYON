import AppKit
import WebKit

@_silgen_name("tac_fixture_menu_count")
private func fixtureMenuCount(_ event: UnsafePointer<CChar>) -> Int64

extension AppDelegate {
    fileprivate var fixtureView: WKWebView? { webView }
    fileprivate var fixtureWindow: NSWindow? { window }
}

@MainActor
private final class MenuActionCheck {
    private let delegate: AppDelegate

    init(_ delegate: AppDelegate) { self.delegate = delegate }

    func run() async throws {
        let view = try await ready(nil)
        let owner = "00000000-0000-4000-8000-000000000011"
        try require(!request(owner, "init", "").contains("error"), "Page service init failed")
        _ = request(owner, "call", "visit")
        _ = try await view.evaluateJavaScript("globalThis.__menuMarker = 41")
        let url = view.url
        let show = try item("View", "show-running")
        try require(enabled(show), "Action with a live recipient is disabled")
        try perform(show)
        try require(count("forwarded") == 1 && count("chosen") == 1, "Menu action did not reach the page service")
        try require(request(owner, "get", "section") == "{\"value\":\"running\"}", "Page service did not receive the section")
        try require(request(owner, "get", "visits") == "{\"value\":1}", "Menu action reset page state")
        try require(view.url == url && !view.isLoading, "Menu action navigated")
        let marker = try await view.evaluateJavaScript("String(globalThis.__menuMarker)") as? String
        try require(marker == "41", "Menu action reloaded the document")
        try await navigate(view, "/help", "Help")
        try require(!enabled(show), "Action stayed enabled without a recipient")
        try perform(show)
        try require(count("forwarded") == 1 && count("refused") == 0, "Disabled action called the application service")
        try require(enabled(try item("File", "refresh")), "Application action without an event was disabled")
        try perform(try item("File", "refresh"))
        try require(count("refreshed") == 1, "Application action did not run")
        try await closeAndReopen(view, show)
        delegate.applicationWillTerminate(Notification(name: NSApplication.willTerminateNotification))
        delegate.applicationWillTerminate(Notification(name: NSApplication.willTerminateNotification))
        try require(count("stopped") == 1, "Quit cleanup did not run exactly once")
        FileHandle.standardOutput.write(Data("menu action, disabled recipient, Close Window, reopen and Quit passed\n".utf8))
    }

    private func closeAndReopen(_ view: WKWebView, _ show: NSMenuItem) async throws {
        let first = delegate.fixtureWindow
        let close = try titled("File", "Close Window")
        try require(enabled(close) && close.target as? NSWindow === first && close.keyEquivalent == "w", "Close Window does not target the owned window")
        try perform(close)
        try require(delegate.fixtureWindow == nil && delegate.fixtureView == nil, "Close Window kept the window")
        try require(count("stopped") == 0, "Closing the last window stopped the application service")
        try require(!enabled(try titled("File", "Close Window")), "Close Window is enabled with no window")
        try require(!enabled(try item("View", "show-running")), "Page action is enabled with no window")
        try perform(try item("File", "refresh"))
        try require(count("refreshed") == 2, "Application action failed with no window")
        _ = delegate.applicationShouldHandleReopen(NSApplication.shared, hasVisibleWindows: false)
        _ = try await ready(view)
        try require(delegate.fixtureWindow !== first, "Reopen reused the closed window")
        let fresh = "00000000-0000-4000-8000-000000000012"
        try require(!request(fresh, "init", "").contains("error"), "Fresh page init failed")
        let reopened = try item("View", "show-running")
        try require(reopened.target !== show.target, "Reopen kept the old menu owner")
        try require(enabled(reopened), "Reopened page action is disabled")
        try perform(reopened)
        try require(request(fresh, "get", "section") == "{\"value\":\"running\"}" && count("chosen") == 2, "Reopened page did not receive the action")
        try require(count("created") == 1, "Reopen created another application service")
    }

    private func ready(_ previous: WKWebView?) async throws -> WKWebView {
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            if let view = delegate.fixtureView, view !== previous, !view.isLoading, view.url != nil { return view }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw failure("Generated host did not finish its navigation")
    }

    private func navigate(_ view: WKWebView, _ path: String, _ title: String) async throws {
        guard let url = URL(string: "tachyon-app://bundle" + path) else { throw failure("Invalid fixture route") }
        view.load(URLRequest(url: url))
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            if view.url == url && !view.isLoading && view.title == title { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw failure("Navigation failed: " + path)
    }

    private func item(_ group: String, _ id: String) throws -> NSMenuItem {
        let found = try menu(group).items.first { (item: NSMenuItem) -> Bool in item.identifier?.rawValue == id }
        guard let found else { throw failure("Missing menu item " + id) }
        return found
    }

    private func titled(_ group: String, _ title: String) throws -> NSMenuItem {
        let found = try menu(group).items.first { (item: NSMenuItem) -> Bool in item.title == title }
        guard let found else { throw failure("Missing menu item " + title) }
        return found
    }

    private func menu(_ title: String) throws -> NSMenu {
        let group = NSApplication.shared.mainMenu?.items.first { (item: NSMenuItem) -> Bool in item.title == title }
        guard let menu = group?.submenu else { throw failure("Missing menu " + title) }
        return menu
    }

    private func enabled(_ item: NSMenuItem) -> Bool {
        item.menu?.update()
        return item.isEnabled
    }

    private func perform(_ item: NSMenuItem) throws {
        guard let action = item.action else { throw failure("Menu item has no action: " + item.title) }
        _ = NSApplication.shared.sendAction(action, to: item.target, from: item)
    }

    private func request(_ owner: String, _ operation: String, _ name: String) -> String {
        let payload = "{\"route\":\"/\",\"service\":\"section\",\"owner\":\"" + owner + "\",\"op\":\"" + operation + "\",\"name\":\"" + name + "\",\"args\":[]}"
        return tacRustInvoke(payload)
    }

    private func count(_ event: String) -> Int64 {
        event.withCString { (pointer: UnsafePointer<CChar>) -> Int64 in fixtureMenuCount(pointer) }
    }

    private func require(_ valid: Bool, _ message: String) throws {
        if !valid { throw failure(message) }
    }

    private func failure(_ message: String) -> NSError {
        NSError(domain: "NativeMenuActionCheck", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}

@main
private struct MenuActionCheckMain {
    static func main() {
        let application = NSApplication.shared
        let delegate = AppDelegate()
        application.delegate = delegate
        application.setActivationPolicy(.regular)
        Task { @MainActor () -> Void in
            do { try await MenuActionCheck(delegate).run(); exit(0) }
            catch { FileHandle.standardError.write(Data(("menu action fixture failed: " + error.localizedDescription + "\n").utf8)); exit(1) }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 30) { () -> Void in
            FileHandle.standardError.write(Data("menu action fixture deadline\n".utf8)); exit(1)
        }
        withExtendedLifetime(delegate) { (_: AppDelegate) -> Void in application.run() }
    }
}
