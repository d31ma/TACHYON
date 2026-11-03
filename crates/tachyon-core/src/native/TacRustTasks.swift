final class TacRustTasks: @unchecked Sendable {
    static let shared = TacRustTasks()
    static let resume = "{\"op\":\"host-resume\"}"
    static let dispatch: @convention(c) () -> Void = { TacRustTasks.shared.post() }

    private let lock = NSLock()
    private var posted = false

    static func install() {
        tacRustNativeSetDispatch(dispatch)
        tacRustNativeSetHttpSender(TacRustHttp.send)
    }

    private func post() {
        lock.lock()
        let first = !posted
        posted = true
        lock.unlock()
        guard first else { return }
        DispatchQueue.main.async { self.run() }
    }

    private func run() {
        lock.lock()
        posted = false
        lock.unlock()
        _ = tacRustInvoke(Self.resume)
    }
}
