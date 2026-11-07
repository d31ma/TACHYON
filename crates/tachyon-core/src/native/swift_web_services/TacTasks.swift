@_extern(wasm, module: "tachyon", name: "tac_host_task_done")
func tacHostTaskDone(_ task: Int32, _ pointer: UnsafePointer<UInt8>?, _ length: Int32)

enum TacTasks {
    static let mostTasks = 256
    static let tooMany = "{\"error\":\"The page runs too many service tasks at once.\",\"code\":\"CAPACITY\"}"
    static let cancelled = "{\"error\":\"The service was removed before its task finished.\",\"code\":\"CANCELLED\"}"
    @TaskLocal static var running: TacRunningTask?
    nonisolated(unsafe) static var last = 0
    nonisolated(unsafe) static var parked: [Int: TacRunningTask] = [:]

    static func begin(_ owner: String, _ work: @escaping ([Any?]) async throws -> Any?, _ arguments: [Any?]) -> String {
        guard parked.count < mostTasks else { return tooMany }
        last += 1
        let started = TacRunningTask(last, owner)
        started.task = Task {
            await TacTasks.$running.withValue(started) { await started.finish { try await work(arguments) } }
        }
        run(started)
        if let answer = started.answer { return answer }
        parked[started.id] = started
        return "{\"task\":\(started.id)}"
    }

    static func resume(_ waiting: TacRunningTask, _ step: () -> Void) {
        guard parked[waiting.id] === waiting else { return }
        let activation = TacNativeState.activations[waiting.owner]
        TacNativeState.handling = true
        TacNativeState.executing = activation
        step()
        run(waiting)
        if let activation { tacDrain(activation) }
        TacNativeState.executing = nil
        TacNativeState.handling = false
        guard let answer = waiting.answer else { return }
        parked.removeValue(forKey: waiting.id)
        done(waiting.id, answer)
    }

    static func cancel(_ owner: String?) {
        let removed = parked.values.filter { owner == nil || $0.owner == owner }.sorted { $0.id < $1.id }
        for waiting in removed {
            parked.removeValue(forKey: waiting.id)
            waiting.task?.cancel()
            TacHttpTickets.stop(waiting)
            run(waiting)
            done(waiting.id, cancelled)
        }
    }

    private static func run(_ running: TacRunningTask) {
        try? MainActor.executor.runUntil { running.answer != nil }
    }

    private static func done(_ id: Int, _ answer: String) {
        var text = answer
        text.withUTF8 { tacHostTaskDone(Int32(id), $0.baseAddress, Int32($0.count)) }
    }
}
