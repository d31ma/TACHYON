import AppKit
import WebKit

@_silgen_name("tac_fixture_application_count")
private func fixtureCount(_ event: UnsafePointer<CChar>) -> Int64

extension AppDelegate {
    fileprivate var fixtureView: WKWebView? { webView }
}

@MainActor
private final class ApplicationCheck {
    private let delegate: AppDelegate

    init(_ delegate: AppDelegate) { self.delegate = delegate }

    func run() async throws {
        let view = try await ready()
        try require(count("created") == 1 && count("started") == 1, "Host did not eagerly start exactly one collector")
        let old = "00000000-0000-4000-8000-000000000001"
        try require(!request(old, "init", "").contains("error"), "First reader init failed")
        _ = request(old, "call", "increment")
        try require(request(old, "call", "read") == "{\"value\":1}", "Reader increment failed")
        try await navigate(view, "/help", "Help")
        try require(count("stopped") == 0 && count("joined") == 0, "Help stopped the application worker")
        try require(request(old, "call", "read").contains("error"), "Old page invocation survived navigation")
        try await navigate(view, "/", "Home")
        let fresh = "00000000-0000-4000-8000-000000000002"
        try require(!request(fresh, "init", "").contains("error"), "Fresh reader init failed")
        try require(request(fresh, "call", "read") == "{\"value\":1}", "Application state did not survive Home/Help/Home")
        try require(count("created") == 1, "Navigation created another collector")
        try await navigate(view, "/help", "Help")
        delegate.applicationWillTerminate(Notification(name: NSApplication.willTerminateNotification))
        try require(count("joined") == 1 && count("stopped") == 1, "Quit from Help did not join exactly once")
        delegate.applicationWillTerminate(Notification(name: NSApplication.willTerminateNotification))
        try require(count("joined") == 1 && count("stopped") == 1, "Repeated shutdown ran cleanup twice")
        try require(!tacRustNativeApplicationStart(), "Retired application revived")
        FileHandle.standardOutput.write(Data("generated AppDelegate Home/Help/Home and joined Quit passed\n".utf8))
    }

    private func ready() async throws -> WKWebView {
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            if let view = delegate.fixtureView, !view.isLoading, view.url != nil { return view }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw failure("Generated host did not finish its initial navigation")
    }

    private func navigate(_ view: WKWebView, _ path: String, _ title: String) async throws {
        guard let url = URL(string: "tachyon-app://bundle" + path) else { throw failure("Invalid fixture route") }
        view.load(URLRequest(url: url))
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            if view.url == url && !view.isLoading && view.title == title { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw failure("Navigation failed: " + path + " title=" + (view.title ?? "nil"))
    }

    private func request(_ owner: String, _ operation: String, _ name: String) -> String {
        let payload = "{\"route\":\"/\",\"service\":\"reader\",\"owner\":\"" + owner + "\",\"op\":\"" + operation + "\",\"name\":\"" + name + "\",\"args\":[]}"
        return tacRustInvoke(payload)
    }

    private func count(_ event: String) -> Int64 {
        event.withCString { (pointer: UnsafePointer<CChar>) -> Int64 in fixtureCount(pointer) }
    }

    private func require(_ valid: Bool, _ message: String) throws {
        if !valid { throw failure(message) }
    }

    private func failure(_ message: String) -> NSError {
        NSError(domain: "NativeApplicationCheck", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}

@main
private struct ApplicationCheckMain {
    static func main() {
        let application = NSApplication.shared
        let delegate = AppDelegate()
        application.delegate = delegate
        application.setActivationPolicy(.regular)
        Task { @MainActor () -> Void in
            do { try await ApplicationCheck(delegate).run(); exit(0) }
            catch { FileHandle.standardError.write(Data(("application fixture failed: " + error.localizedDescription + "\n").utf8)); exit(1) }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 30) { () -> Void in
            FileHandle.standardError.write(Data("application fixture deadline\n".utf8)); exit(1)
        }
        withExtendedLifetime(delegate) { (_: AppDelegate) -> Void in application.run() }
    }
}
