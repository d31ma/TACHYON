import AppKit
import WebKit

extension AppDelegate {
    fileprivate var fixtureView: WKWebView? { webView }
}

@MainActor
private final class HostLinkCheck {
    private let delegate: AppDelegate

    init(_ delegate: AppDelegate) { self.delegate = delegate }

    func run() async throws {
        let view = try await ready()
        try await follow(view, "https://example.com/settings?tab=2", "tachyon-app://bundle/settings?tab=2", true)
        try await follow(view, "https://example.com/missing", "tachyon-app://bundle/index.html", true)
        try await follow(view, "https://example.org/settings", "tachyon-app://bundle/index.html", false)
        let other = NSUserActivity(activityType: "org.tachyon.handoff")
        other.webpageURL = URL(string: "https://example.com/settings")
        try require(!handle(other), "A non-browsing activity opened the app")
        FileHandle.standardOutput.write(Data("macOS universal link delegate passed\n".utf8))
    }

    private func follow(_ view: WKWebView, _ link: String, _ expected: String, _ opens: Bool) async throws {
        let activity = NSUserActivity(activityType: NSUserActivityTypeBrowsingWeb)
        activity.webpageURL = URL(string: link)
        try require(handle(activity) == opens, "The delegate answered the wrong way for " + link)
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            if view.url?.absoluteString == expected && !view.isLoading { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw failure("Link " + link + " left the page at " + (view.url?.absoluteString ?? "nil"))
    }

    private func handle(_ activity: NSUserActivity) -> Bool {
        let application = NSApplication.shared
        let answer = application.delegate?.application?(application, continue: activity) { (_: [any NSUserActivityRestoring]) -> Void in }
        return answer ?? false
    }

    private func ready() async throws -> WKWebView {
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            if let view = delegate.fixtureView, !view.isLoading, view.url != nil { return view }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw failure("Generated host did not finish its initial navigation")
    }

    private func require(_ valid: Bool, _ message: String) throws {
        if !valid { throw failure(message) }
    }

    private func failure(_ message: String) -> NSError {
        NSError(domain: "NativeHostLinkCheck", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}

@main
private struct HostLinkCheckMain {
    static func main() {
        let application = NSApplication.shared
        let delegate = AppDelegate()
        application.delegate = delegate
        application.setActivationPolicy(.regular)
        Task { @MainActor () -> Void in
            do { try await HostLinkCheck(delegate).run(); exit(0) }
            catch { FileHandle.standardError.write(Data(("host link fixture failed: " + error.localizedDescription + "\n").utf8)); exit(1) }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 30) { () -> Void in
            FileHandle.standardError.write(Data("host link fixture deadline\n".utf8)); exit(1)
        }
        withExtendedLifetime(delegate) { (_: AppDelegate) -> Void in application.run() }
    }
}
