enum TacOwnerThread {
    static let pause = DispatchTimeInterval.milliseconds(1)

    static func wait(for finished: DispatchSemaphore) {
        while finished.wait(timeout: .now() + pause) == .timedOut {
            RunLoop.current.run(mode: .default, before: Date())
        }
    }
}
