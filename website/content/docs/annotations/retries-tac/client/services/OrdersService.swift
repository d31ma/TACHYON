final class OrdersService {
    struct GatewayUnavailable: Error {}

    var behaviors: BehaviorContext!
    private static let firstWorkingAttempt = 3

    var attempts: Int = 0

    @OnStart
    func open() {
        behaviors = Self.openOrders()
    }

    @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
    func submit(_ id: BehaviorJSON) async throws -> BehaviorJSON {
        attempts += 1
        try failFirstTwoAttempts()
        return reportOf(id)
    }

    @MaxAtOnce({"limit":2,"policy":"block"})
    func report(_ id: BehaviorJSON) async throws -> BehaviorJSON {
        return reportOf(id)
    }

    private func reportOf(_ id: BehaviorJSON) -> BehaviorJSON {
        .object(["id": id, "attempts": .number(Double(attempts))])
    }

    private func failFirstTwoAttempts() throws {
        if attempts < Self.firstWorkingAttempt { throw GatewayUnavailable() }
    }

    private static func openOrders() -> BehaviorContext? {
        try? Behaviors.create(OrdersSetup.self, partition: "orders", signal: BehaviorSignal())
    }
}
