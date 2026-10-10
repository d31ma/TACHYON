import AppKit
import ObjectiveC
import WebKit

extension AppDelegate {
    fileprivate var fixtureView: WKWebView? { webView }
    fileprivate var fixtureWindow: NSWindow? { window }
    fileprivate var fixtureTitleBar: MacOsTitleBar? { titleBar }
}

extension NSWindow {
    @objc fileprivate func fixturePerformDrag(with event: NSEvent) {
        TitleBarCheck.drags += 1
    }
}

@MainActor
private final class TitleBarCheck {
    nonisolated(unsafe) static var drags = 0

    private let delegate: AppDelegate
    private let screenshots: String
    private let applicationName = "Title Bar Check"

    init(_ delegate: AppDelegate) {
        self.delegate = delegate
        screenshots = ProcessInfo.processInfo.environment["TAC_TITLE_BAR_SCREENSHOTS"] ?? NSTemporaryDirectory()
    }

    func run() async throws {
        TitleBarCheck.spyOnWindowDrags()
        let view = try await ready()
        let window = try owned()
        try checkStyle(window, view)
        try checkButtons(window)
        let expected = try expectedInsets(window)
        let light = try await appearance(window, view, .aqua, false, expected, "light")
        let dark = try await appearance(window, view, .darkAqua, true, expected, "dark")
        try require(light.brightnessComponent > dark.brightnessComponent + 0.3, "Dark mode did not darken the title bar area")
        try await checkDragRegions(window, view)
        try await checkDoubleClick(window, view)
        try await checkResize(window, view, expected)
        try await checkFullScreen(window, view, expected)
        try checkTitle(window)
        try await checkMinimizeAndClose(window)
        FileHandle.standardOutput.write(Data("integrated title bar style, buttons, insets, light, dark, drag regions, double-click, resize, full screen, title and close passed\n".utf8))
    }

    private static func spyOnWindowDrags() {
        let original = class_getInstanceMethod(NSWindow.self, #selector(NSWindow.performDrag(with:)))
        let spy = class_getInstanceMethod(NSWindow.self, #selector(NSWindow.fixturePerformDrag(with:)))
        if let original, let spy { method_exchangeImplementations(original, spy) }
    }

    private func checkStyle(_ window: NSWindow, _ view: WKWebView) throws {
        let mask = window.styleMask
        try require(mask.contains(.fullSizeContentView), "Window has no full-size content view")
        try require(mask.isSuperset(of: [.titled, .closable, .miniaturizable, .resizable]), "Window lost a standard style")
        try require(window.titlebarAppearsTransparent, "Title bar is not transparent")
        try require(window.titleVisibility == .hidden, "Title text is visible")
        try require(window.titlebarSeparatorStyle == .none, "Title bar draws a separator")
        try require(window.title == applicationName, "Window title is not the application name")
        try require(window.accessibilityTitle() == applicationName, "Window has no accessible title")
        try require(!window.isExcludedFromWindowsMenu, "Window is missing from the Window menu")
        try require(window.isMovable, "Window is not movable")
        try require(view.frame.size == window.frame.size, "Page does not fill the window under the title bar")
    }

    private func checkButtons(_ window: NSWindow) throws {
        let frameView = try theme(window)
        for type in [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton] {
            guard let button = window.standardWindowButton(type) else { throw failure("Missing window button") }
            try require(button.isEnabled && !button.isHidden && button.window === window, "Window button is not usable")
            let center = button.convert(NSPoint(x: button.bounds.midX, y: button.bounds.midY), to: nil)
            let hit = frameView.hitTest(frameView.convert(center, from: nil))
            try require(hit === button, "Page covers a window button")
        }
        guard TitleBarExpectation.buttonLeading >= 0, let close = window.standardWindowButton(.closeButton) else { return }
        let frame = close.convert(close.bounds, to: nil)
        let top = window.frame.height - frame.maxY
        try require(Int(frame.minX) == TitleBarExpectation.buttonLeading && Int(top) == TitleBarExpectation.buttonTop,
                    "Close button is not at the manifest inset: \(frame) top \(top)")
    }

    private func expectedInsets(_ window: NSWindow) throws -> MacOsTitleBarInsets {
        if TitleBarExpectation.height > 0 {
            return MacOsTitleBarInsets(height: TitleBarExpectation.height, leading: TitleBarExpectation.leading, trailing: 0)
        }
        guard let close = window.standardWindowButton(.closeButton), let zoom = window.standardWindowButton(.zoomButton) else {
            throw failure("Missing window button")
        }
        let strip = window.frame.height - window.contentLayoutRect.maxY
        let leading = zoom.convert(zoom.bounds, to: nil).maxX + close.convert(close.bounds, to: nil).minX
        try require(strip >= 20 && leading >= 40, "Native title bar metrics are implausible")
        return MacOsTitleBarInsets(height: Int(strip.rounded(.up)), leading: Int(leading.rounded(.up)), trailing: 0)
    }

    private func appearance(_ window: NSWindow, _ view: WKWebView, _ name: NSAppearance.Name, _ dark: Bool,
                            _ expected: MacOsTitleBarInsets, _ label: String) async throws -> NSColor {
        NSApp.appearance = NSAppearance(named: name)
        try await until("page appearance " + label) {
            try await self.script(view, "String(matchMedia('(prefers-color-scheme: dark)').matches)") == String(dark)
        }
        try await expectInsets(view, expected, label)
        try await settle()
        let image = try capture(window, label)
        let strip = try sample(image, window, NSPoint(x: window.frame.width / 2, y: 6))
        let bar = try sample(image, window, NSPoint(x: window.frame.width / 2, y: 70))
        try require(similar(strip, bar), "Title bar strip does not match the page in \(label) mode: \(strip) vs \(bar)")
        return strip
    }

    private func checkDragRegions(_ window: NSWindow, _ view: WKWebView) async throws {
        try await until("drag regions reported") {
            try await self.script(view, "String(globalThis.__tachyonTitleBar._sent.includes('\"drag\":[['))") == "true"
        }
        try await until("drag regions received") {
            self.delegate.fixtureTitleBar?.contains(self.event(window, NSPoint(x: 300, y: 60), 1, .leftMouseDown), view) == true
        }
        let action = try await center(view, "action")
        try await focus(window)
        let before = window.frame
        click(window, NSPoint(x: 300, y: 60), 1)
        try require(TitleBarCheck.drags == 1, "Drag region did not start a window drag")
        click(window, action, 1)
        try require(TitleBarCheck.drags == 1, "Button inside the drag region started a window drag")
        click(window, NSPoint(x: 300, y: 200), 1)
        try require(TitleBarCheck.drags == 1, "Page content started a window drag")
        let clicked = focusState(window)
        do {
            try await until("page clicks") { try await self.pageClicks(view) == "action,main 1" }
        } catch {
            let found = (try? await pageClicks(view)) ?? "nothing"
            throw failure("Timed out waiting for page clicks: found \(found), focus during clicks \(clicked)")
        }
        try require(window.frame == before, "A click moved the window")
    }

    private func focus(_ window: NSWindow) async throws {
        if NSApp.isActive && window.isKeyWindow { return }
        FileHandle.standardError.write(Data(("title bar fixture regaining focus: " + focusState(window) + "\n").utf8))
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        do {
            try await until("key window") { NSApp.isActive && window.isKeyWindow }
        } catch {
            throw failure("Window is not key before page clicks: " + focusState(window))
        }
    }

    private func focusState(_ window: NSWindow) -> String {
        let frontmost = NSWorkspace.shared.frontmostApplication?.bundleIdentifier ?? "none"
        return "variant=\(TitleBarExpectation.variant), active=\(NSApp.isActive), key=\(window.isKeyWindow), frontmost=\(frontmost)"
    }

    private func pageClicks(_ view: WKWebView) async throws -> String {
        let downs = try await script(view, "globalThis.__downs.join(',')")
        let clicks = try await script(view, "String(globalThis.__clicks)")
        return downs + " " + clicks
    }

    private func checkDoubleClick(_ window: NSWindow, _ view: WKWebView) async throws {
        let before = window.frame
        click(window, NSPoint(x: 300, y: 60), 1)
        click(window, NSPoint(x: 300, y: 60), 2)
        try await until("double-click zoom") { window.frame != before }
        try await settle()
        try require(window.isZoomed, "Double-click did not follow the zoom setting")
        click(window, NSPoint(x: 300, y: 60), 1)
        click(window, NSPoint(x: 300, y: 60), 2)
        try await until("double-click restore") { window.frame == before }
        try await settle()
        try require(try await script(view, "globalThis.__downs.join(',')") == "action,main", "Double-click reached the page")
    }

    private func checkResize(_ window: NSWindow, _ view: WKWebView, _ expected: MacOsTitleBarInsets) async throws {
        let before = window.frame
        let larger = NSRect(x: before.minX, y: before.minY - 120, width: before.width + 200, height: before.height + 120)
        window.setFrame(larger, display: true)
        try await until("resize") {
            try await self.script(view, "String(innerWidth)") == String(Int(larger.width))
        }
        try require(view.frame.size == window.frame.size, "Page does not fill the resized window")
        try await expectInsets(view, expected, "resized")
        try checkButtons(window)
        window.setFrame(before, display: true)
        try await until("resize back") {
            try await self.script(view, "String(innerWidth)") == String(Int(before.width))
        }
    }

    private func checkFullScreen(_ window: NSWindow, _ view: WKWebView, _ expected: MacOsTitleBarInsets) async throws {
        guard let zoom = window.standardWindowButton(.zoomButton) else { throw failure("Missing zoom button") }
        zoom.performClick(nil)
        try await until("full screen", 10) { window.styleMask.contains(.fullScreen) }
        try await expectInsets(view, MacOsTitleBarInsets.zero, "full screen")
        try await settle(2)
        _ = try capture(window, "fullscreen")
        try await navigate(view, "/help", "Help")
        try await expectInsets(view, MacOsTitleBarInsets.zero, "full screen after navigation")
        try await navigate(view, "/", "Home")
        window.toggleFullScreen(nil)
        try await until("leave full screen", 10) { !window.styleMask.contains(.fullScreen) }
        try await settle(2)
        try await expectInsets(view, expected, "after full screen")
        try checkButtons(window)
    }

    private func checkTitle(_ window: NSWindow) throws {
        NativeBridge.retitle("{\"title\":\"\"}")
        try require(window.title == applicationName, "An empty title removed the accessible window name")
        NativeBridge.retitle("{\"title\":\"Inbox\"}")
        try require(window.title == "Inbox" && window.titleVisibility == .hidden, "Window title did not change")
    }

    private func checkMinimizeAndClose(_ window: NSWindow) async throws {
        window.standardWindowButton(.miniaturizeButton)?.performClick(nil)
        try await until("minimize", 10) { window.isMiniaturized }
        window.deminiaturize(nil)
        try await until("restore", 10) { !window.isMiniaturized }
        window.standardWindowButton(.closeButton)?.performClick(nil)
        try await until("close") { self.delegate.fixtureWindow == nil }
    }

    private func expectInsets(_ view: WKWebView, _ expected: MacOsTitleBarInsets, _ label: String) async throws {
        let wanted = "\(expected.height)px \(expected.leading)px \(expected.trailing)px"
        let read = "['--tac-titlebar-height','--tac-titlebar-leading-inset','--tac-titlebar-trailing-inset'].map(name => getComputedStyle(document.documentElement).getPropertyValue(name).trim()).join(' ')"
        do {
            try await until("insets " + label) { try await self.script(view, read) == wanted }
        } catch {
            let found = (try? await script(view, read)) ?? "nothing"
            throw failure("Insets \(label): wanted \(wanted), found \(found)")
        }
    }

    private func ready() async throws -> WKWebView {
        try await until("navigation") {
            guard let view = self.delegate.fixtureView else { return false }
            return !view.isLoading && view.title == "Home"
        }
        guard let view = delegate.fixtureView else { throw failure("No web view") }
        return view
    }

    private func owned() throws -> NSWindow {
        guard let window = delegate.fixtureWindow else { throw failure("No window") }
        return window
    }

    private func theme(_ window: NSWindow) throws -> NSView {
        guard let frameView = window.contentView?.superview else { throw failure("No window frame view") }
        return frameView
    }

    private func navigate(_ view: WKWebView, _ path: String, _ title: String) async throws {
        guard let url = URL(string: "tachyon-app://bundle" + path) else { throw failure("Invalid route") }
        view.load(URLRequest(url: url))
        try await until("navigate " + path) { view.url == url && !view.isLoading && view.title == title }
    }

    private func center(_ view: WKWebView, _ id: String) async throws -> NSPoint {
        let text = try await script(view, "(b => b.left + b.width / 2 + ',' + (b.top + b.height / 2))(document.getElementById('\(id)').getBoundingClientRect())")
        let parts = text.split(separator: ",").compactMap { (part: Substring) -> Double? in Double(part) }
        guard parts.count == 2 else { throw failure("No element " + id) }
        return NSPoint(x: parts[0], y: parts[1])
    }

    private func event(_ window: NSWindow, _ top: NSPoint, _ count: Int, _ type: NSEvent.EventType) -> NSEvent {
        let location = NSPoint(x: top.x, y: window.frame.height - top.y)
        return NSEvent.mouseEvent(with: type, location: location, modifierFlags: [],
                                  timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                                  context: nil, eventNumber: 0, clickCount: count, pressure: 1)!
    }

    private func click(_ window: NSWindow, _ top: NSPoint, _ count: Int) {
        window.sendEvent(event(window, top, count, .leftMouseDown))
        window.sendEvent(event(window, top, count, .leftMouseUp))
    }

    private func capture(_ window: NSWindow, _ label: String) throws -> NSBitmapImageRep {
        let path = (screenshots as NSString).appendingPathComponent("titlebar-\(TitleBarExpectation.variant)-\(label).png")
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
        process.arguments = ["-x", "-o", "-l\(window.windowNumber)", path]
        try process.run()
        process.waitUntilExit()
        guard process.terminationStatus == 0, let data = FileManager.default.contents(atPath: path),
              let image = NSBitmapImageRep(data: data) else { throw failure("Screenshot failed: " + path) }
        FileHandle.standardOutput.write(Data(("screenshot " + path + "\n").utf8))
        return image
    }

    private func sample(_ image: NSBitmapImageRep, _ window: NSWindow, _ top: NSPoint) throws -> NSColor {
        let scale = CGFloat(image.pixelsWide) / window.frame.width
        let color = image.colorAt(x: Int(top.x * scale), y: Int(top.y * scale))?.usingColorSpace(.sRGB)
        guard let color else { throw failure("Screenshot has no pixel") }
        return color
    }

    private func similar(_ first: NSColor, _ second: NSColor) -> Bool {
        abs(first.redComponent - second.redComponent) < 0.03
            && abs(first.greenComponent - second.greenComponent) < 0.03
            && abs(first.blueComponent - second.blueComponent) < 0.03
    }

    private func script(_ view: WKWebView, _ source: String) async throws -> String {
        let value = try await view.evaluateJavaScript(source)
        return value as? String ?? ""
    }

    private func settle(_ seconds: Double = 0.5) async throws {
        try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
    }

    private func until(_ label: String, _ seconds: Double = 5, _ condition: () async throws -> Bool) async throws {
        let deadline = Date().addingTimeInterval(seconds)
        while Date() < deadline {
            if try await condition() { return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        throw failure("Timed out waiting for " + label)
    }

    private func require(_ valid: Bool, _ message: String) throws {
        if !valid { throw failure(message) }
    }

    private func failure(_ message: String) -> NSError {
        NSError(domain: "NativeTitleBarCheck", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}

@main
private struct TitleBarCheckMain {
    static func main() {
        let application = NSApplication.shared
        let delegate = AppDelegate()
        application.delegate = delegate
        application.setActivationPolicy(.regular)
        Task { @MainActor () -> Void in
            do { try await TitleBarCheck(delegate).run(); exit(0) }
            catch { FileHandle.standardError.write(Data(("title bar fixture failed: " + error.localizedDescription + "\n").utf8)); exit(1) }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 90) { () -> Void in
            FileHandle.standardError.write(Data("title bar fixture deadline\n".utf8)); exit(1)
        }
        withExtendedLifetime(delegate) { (_: AppDelegate) -> Void in application.run() }
    }
}
