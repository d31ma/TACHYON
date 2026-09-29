final class Clock {
    func nowMs() -> Double {
        tacHostEpochMilliseconds()
    }

    func monotonicMs() -> Double {
        tacHostMonotonicMilliseconds()
    }

    func timeZone() throws -> String {
        try TacWebHost.text("clock", "timeZone")
    }
}
