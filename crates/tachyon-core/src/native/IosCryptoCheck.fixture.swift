extension RootViewController {
    fileprivate var fixtureView: WKWebView? { webView }
}

@MainActor
private final class IosCryptoCheck {
    static let checks: [String: [String]] = __CRYPTO_CHECKS__

    static func start() {
        Task { @MainActor () -> Void in
            do { try await run(); exit(0) }
            catch { FileHandle.standardOutput.write(Data(("crypto fixture failed: " + error.localizedDescription + "\n").utf8)); exit(1) }
        }
    }

    private static func run() async throws {
        let view = try await ready()
        let phase = CommandLine.arguments.dropFirst().first ?? "first"
        for script in checks[phase] ?? [] {
            let value = try await view.callAsyncJavaScript(script, arguments: [:], contentWorld: .page)
            FileHandle.standardOutput.write(Data("crypto check \(phase): \(value as? String ?? String(describing: value))\n".utf8))
        }
        FileHandle.standardOutput.write(Data("crypto checks done\n".utf8))
    }

    private static func ready() async throws -> WKWebView {
        let deadline = Date().addingTimeInterval(30)
        while Date() < deadline {
            let root = (UIApplication.shared.delegate as? AppDelegate)?.window?.rootViewController as? RootViewController
            if let view = root?.fixtureView, !view.isLoading, view.title == "Home" { return view }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw NSError(domain: "IosCryptoCheck", code: 1, userInfo: [NSLocalizedDescriptionKey: "Generated host did not finish its initial navigation"])
    }
}

@main
private struct IosCryptoCheckMain {
    static func main() {
        NotificationCenter.default.addObserver(forName: UIApplication.didFinishLaunchingNotification, object: nil, queue: .main) { _ in
            MainActor.assumeIsolated { IosCryptoCheck.start() }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 90) { () -> Void in
            FileHandle.standardOutput.write(Data("crypto fixture deadline\n".utf8)); exit(1)
        }
        _ = UIApplicationMain(CommandLine.argc, CommandLine.unsafeArgv, nil, NSStringFromClass(AppDelegate.self))
    }
}
