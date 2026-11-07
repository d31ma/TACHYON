final class BehaviorLock {
    private let mutex = Foundation.NSLock()

    func lock() {
        mutex.lock()
    }

    func unlock() {
        mutex.unlock()
    }
}
