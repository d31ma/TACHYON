enum TacPage {
    static let millisecondsPerSecond = 1000.0

    static func epochMilliseconds() -> Double {
        answered("now") ?? Date().timeIntervalSince1970 * millisecondsPerSecond
    }

    static func monotonicMilliseconds() -> Double {
        answered("monotonic") ?? ProcessInfo.processInfo.systemUptime * millisecondsPerSecond
    }

    static func answer(_ api: String, _ call: String, _ arguments: [Any?]) -> [String: Any]? {
        BrowserFakes.reply(api, call)
    }

    private static func answered(_ call: String) -> Double? {
        BrowserFakes.reply("clock", call)["value"] as? Double
    }
}
