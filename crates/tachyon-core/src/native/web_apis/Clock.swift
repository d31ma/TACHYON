final class Clock {
    func nowMs() -> Double {
        TacPage.epochMilliseconds()
    }

    func monotonicMs() -> Double {
        TacPage.monotonicMilliseconds()
    }

    func timeZone() throws -> String {
        try TacWebHost.text("clock", "timeZone")
    }
}
