@MainActor
private final class AppleApplicationState: NSObject {
    static let shared = AppleApplicationState()
    private var state = "foreground"
    private var changes: [Notification.Name: String] = [:]

    func watch(_ changes: [Notification.Name: String]) {
        self.changes = changes
        for name in changes.keys {
            NotificationCenter.default.addObserver(self, selector: #selector(changed(_:)), name: name, object: nil)
        }
    }

    @objc private func changed(_ notification: Notification) {
        guard let next = changes[notification.name], next != state else { return }
        state = next
        Telemetry.shared.record("application." + next)
        post()
    }

    func post() {
        let finished = hold()
        guard let webView = NativeBridge.shared.webView else { finished(); return }
        webView.evaluateJavaScript("globalThis.__tachyonApplicationState?.('" + state + "')") { _, _ in finished() }
    }

    private func hold() -> @MainActor @Sendable () -> Void {
        #if os(iOS)
        let task = UIApplication.shared.beginBackgroundTask(expirationHandler: nil)
        return {
            Task { @MainActor in
                try? await Task.sleep(nanoseconds: 1_000_000_000)
                UIApplication.shared.endBackgroundTask(task)
            }
        }
        #else
        return {}
        #endif
    }
}
