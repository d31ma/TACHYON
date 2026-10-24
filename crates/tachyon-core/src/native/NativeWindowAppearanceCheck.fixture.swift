import AppKit
import WebKit

extension AppDelegate {
    fileprivate var fixtureView: WKWebView? { webView }
    fileprivate var fixtureWindow: NSWindow? { window }
}

@MainActor
private final class WindowAppearanceCheck {
    private let delegate: AppDelegate
    private let screenshots: String

    init(_ delegate: AppDelegate) {
        self.delegate = delegate
        screenshots = ProcessInfo.processInfo.environment["TAC_WINDOW_APPEARANCE_SCREENSHOTS"] ?? NSTemporaryDirectory()
    }

    func run() async throws {
        let view = try await ready()
        guard let window = delegate.fixtureWindow, let page = view as? MacOsTitleBarWebView else { throw failure("No window") }
        try checkBackdrop(page)
        if WindowAppearanceExpectation.fullscreen {
            try await until("full screen", 10) { window.styleMask.contains(.fullScreen) }
            try await settle(2)
            try require(!window.titlebarAppearsTransparent, "A window without theme_color changed its title bar")
            let image = try capture(window, "fullscreen")
            try expect(try sample(image, window, NSPoint(x: window.frame.width / 2, y: window.frame.height / 2)),
                       WindowAppearanceExpectation.background, "full-screen page backdrop")
            FileHandle.standardOutput.write(Data("window appearance passed: display fullscreen started full screen over the background_color backdrop\n".utf8))
            return
        }
        try checkTitleBar(window)
        try await checkPageAppearance(view, window)
        try await settle()
        let image = try capture(window, "themed")
        let bar = window.frame.height - window.contentLayoutRect.maxY
        try require(bar >= 20, "The window has no title bar strip")
        try expect(try sample(image, window, NSPoint(x: window.frame.width * 0.85, y: bar / 2)),
                   WindowAppearanceExpectation.theme, "title bar")
        try expect(try sample(image, window, NSPoint(x: window.frame.width / 2, y: bar + (window.frame.height - bar) / 2)),
                   WindowAppearanceExpectation.background, "page backdrop")
        FileHandle.standardOutput.write(Data("window appearance passed: theme_color colored the title bar with white title text and background_color sat behind the page\n".utf8))
    }

    private func checkBackdrop(_ view: MacOsTitleBarWebView) throws {
        guard let backdrop = view.backdrop?.usingColorSpace(.sRGB), let layer = view.layer?.backgroundColor,
              let painted = NSColor(cgColor: layer)?.usingColorSpace(.sRGB) else {
            throw failure("background_color did not reach the web view")
        }
        try expect(backdrop, WindowAppearanceExpectation.background, "web view backdrop")
        try expect(painted, WindowAppearanceExpectation.background, "web view layer")
    }

    private func checkTitleBar(_ window: NSWindow) throws {
        try require(window.titlebarAppearsTransparent, "theme_color did not open the title bar to the window color")
        guard let color = window.backgroundColor.usingColorSpace(.sRGB) else { throw failure("Window color has no sRGB form") }
        try expect(color, WindowAppearanceExpectation.theme, "window color")
        try require(window.appearance?.name == .darkAqua, "A dark theme_color did not take white title text")
        try require(window.titleVisibility == .visible && window.title == "Appearance Check", "The standard title is hidden")
    }

    private func checkPageAppearance(_ view: WKWebView, _ window: NSWindow) async throws {
        for (name, dark) in [(NSAppearance.Name.aqua, false), (.darkAqua, true), (.aqua, false)] {
            NSApp.appearance = NSAppearance(named: name)
            try await until("page appearance \(dark)") {
                try await self.script(view, "String(matchMedia('(prefers-color-scheme: dark)').matches)") == String(dark)
            }
            try require(window.appearance?.name == .darkAqua, "The page appearance changed the title bar")
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

    private func capture(_ window: NSWindow, _ label: String) throws -> NSBitmapImageRep {
        let path = (screenshots as NSString).appendingPathComponent("window-appearance-\(label).png")
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

    private func expect(_ found: NSColor, _ wanted: [CGFloat], _ label: String) throws {
        let channels = [found.redComponent, found.greenComponent, found.blueComponent].map { (value: CGFloat) -> CGFloat in value * 255 }
        let close = zip(channels, wanted).allSatisfy { (pair: (CGFloat, CGFloat)) -> Bool in abs(pair.0 - pair.1) <= 6 }
        try require(close, "\(label) is \(channels.map { (value: CGFloat) -> Int in Int(value.rounded()) }), not \(wanted.map { (value: CGFloat) -> Int in Int(value) })")
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
        NSError(domain: "NativeWindowAppearanceCheck", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}

@main
private struct WindowAppearanceCheckMain {
    static func main() {
        let application = NSApplication.shared
        let delegate = AppDelegate()
        application.delegate = delegate
        application.setActivationPolicy(.regular)
        Task { @MainActor () -> Void in
            do { try await WindowAppearanceCheck(delegate).run(); exit(0) }
            catch { FileHandle.standardError.write(Data(("window appearance fixture failed: " + error.localizedDescription + "\n").utf8)); exit(1) }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 60) { () -> Void in
            FileHandle.standardError.write(Data("window appearance fixture deadline\n".utf8)); exit(1)
        }
        withExtendedLifetime(delegate) { (_: AppDelegate) -> Void in application.run() }
    }
}
