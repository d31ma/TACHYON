import AppKit
import WebKit

extension AppDelegate {
    fileprivate var fixtureView: WKWebView? { webView }
}

@MainActor
private final class CryptoCheck {
    static let checks: [String: [String]] = __CRYPTO_CHECKS__
    private let delegate: AppDelegate

    init(_ delegate: AppDelegate) { self.delegate = delegate }

    func run() async throws {
        let view = try await ready()
        let phase = CommandLine.arguments.dropFirst().first ?? "first"
        for script in Self.checks[phase] ?? [] {
            let value = try await view.callAsyncJavaScript(script, arguments: [:], contentWorld: .page)
            FileHandle.standardOutput.write(Data("crypto check \(phase): \(value as? String ?? String(describing: value))\n".utf8))
        }
        FileHandle.standardOutput.write(Data("crypto checks done\n".utf8))
    }

    private func ready() async throws -> WKWebView {
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            if let view = delegate.fixtureView, !view.isLoading, view.title == "Home" { return view }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw failure("Generated host did not finish its initial navigation")
    }

    private func failure(_ message: String) -> NSError {
        NSError(domain: "NativeCryptoCheck", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}

@main
private struct CryptoCheckMain {
    static func main() {
        let application = NSApplication.shared
        let delegate = AppDelegate()
        application.delegate = delegate
        application.setActivationPolicy(.regular)
        Task { @MainActor () -> Void in
            do { try await CryptoCheck(delegate).run(); exit(0) }
            catch { FileHandle.standardError.write(Data(("crypto fixture failed: " + error.localizedDescription + "\n").utf8)); exit(1) }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 60) { () -> Void in
            FileHandle.standardError.write(Data("crypto fixture deadline\n".utf8)); exit(1)
        }
        withExtendedLifetime(delegate) { (_: AppDelegate) -> Void in application.run() }
    }
}
