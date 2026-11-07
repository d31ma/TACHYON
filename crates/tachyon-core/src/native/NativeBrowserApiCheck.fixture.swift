import AppKit
import WebKit

extension AppDelegate {
    fileprivate var fixtureView: WKWebView? { webView }
}

@MainActor
private final class BrowserApiCheck {
    private let delegate: AppDelegate

    init(_ delegate: AppDelegate) { self.delegate = delegate }

    func run() async throws {
        let view = try await ready()
        let board = NSPasteboard.general
        let saved = board.string(forType: .string)
        defer {
            board.clearContents()
            if let saved { board.setString(saved, forType: .string) }
        }
        let bridged = try await page(view, "return globalThis.__tachyonBrowserApiBridge.join(',')", [:])
        try require(bridged == "clipboard,share,notifications,geolocation", "Page bridge list: " + bridged)
        let written = try await call(view, "browserApi.clipboard.writeText", "{\"text\":\"tachyon bridge\"}")
        try require(written == "{\"value\":null}", "writeText answered " + written)
        try require(board.string(forType: .string) == "tachyon bridge", "writeText did not reach the pasteboard")
        board.clearContents()
        board.setString("from the Mac", forType: .string)
        let read = try await call(view, "browserApi.clipboard.readText", "{}")
        try require(read == "{\"value\":\"from the Mac\"}", "readText answered " + read)
        let share = try await call(view, "browserApi.share.open", "{\"text\":\"not granted\"}")
        try require(share.contains("not granted"), "An API the app does not grant was answered: " + share)
        let shown = try await call(view, "browserApi.notifications.show", "{\"title\":\"not granted\"}")
        try require(shown.contains("not granted"), "Notifications the app does not grant were answered: " + shown)
        let unknown = try await call(view, "browserApi.clipboard.erase", "{}")
        try require(unknown.contains("could not run"), "An unknown call was answered: " + unknown)
        FileHandle.standardOutput.write(Data("clipboard bridge, refusals and page list passed\n".utf8))
    }

    private func ready() async throws -> WKWebView {
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            if let view = delegate.fixtureView, !view.isLoading, view.title == "Home" { return view }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw failure("Generated host did not finish its initial navigation")
    }

    private func call(_ view: WKWebView, _ capability: String, _ payload: String) async throws -> String {
        try await page(
            view,
            "return await globalThis.__tachyonNativeHostCall(capability, payload)",
            ["capability": capability, "payload": payload]
        )
    }

    private func page(_ view: WKWebView, _ body: String, _ arguments: [String: Any]) async throws -> String {
        let value = try await view.callAsyncJavaScript(body, arguments: arguments, contentWorld: .page)
        return value as? String ?? String(describing: value)
    }

    private func require(_ valid: Bool, _ message: String) throws {
        if !valid { throw failure(message) }
    }

    private func failure(_ message: String) -> NSError {
        NSError(domain: "NativeBrowserApiCheck", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}

@main
private struct BrowserApiCheckMain {
    static func main() {
        let application = NSApplication.shared
        let delegate = AppDelegate()
        application.delegate = delegate
        application.setActivationPolicy(.regular)
        Task { @MainActor () -> Void in
            do { try await BrowserApiCheck(delegate).run(); exit(0) }
            catch { FileHandle.standardError.write(Data(("browser API fixture failed: " + error.localizedDescription + "\n").utf8)); exit(1) }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 30) { () -> Void in
            FileHandle.standardError.write(Data("browser API fixture deadline\n".utf8)); exit(1)
        }
        withExtendedLifetime(delegate) { (_: AppDelegate) -> Void in application.run() }
    }
}
