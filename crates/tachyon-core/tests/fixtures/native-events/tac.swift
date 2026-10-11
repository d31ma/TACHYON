@Lifetime({"value":"prototype"})
class OwnerProbeService {
    var count: Int = 0
    var log: String = ""
    let deaths: Int = TacStore.session("eventDeaths", 0)
    let applicationEventPublisher: ApplicationEventPublisher
    @OnStart
    func start() {
        count = 1
        _ = send()
    }
    @OnStop
    func stop() {
        TacStore.setSession("eventDeaths", deaths + 1)
    }
    func send() -> Int {
        applicationEventPublisher.publishEvent(Pulse(amount: 5), true)
        return 5
    }
    @OnEvent
    func receive(_ value: Pulse) {
        count += value.amount
        _ = relay()
    }
    func relay() -> Int {
        applicationEventPublisher.publishEvent(Relay(amount: 3))
        return 3
    }
    @OnEvent
    func receiveRelay(value: Relay) {
        count += value.amount
    }
    func cycle() -> Int {
        applicationEventPublisher.publishEvent(Cycle(amount: 1))
        return 1
    }
    @OnEvent
    func receiveCycle(_ value: Cycle) {
        count += value.amount
        _ = cycle()
    }
    func isolated() -> [String: Int] {
        applicationEventPublisher.publishEvent(Isolation(value: 1))
        return ["value": 1]
    }
    @OnEvent
    func aMutation(_ value: Isolation) {
        var copy = value
        copy.value = 99
        count += copy.value - 99
    }
    @OnEvent
    func bOriginal(_ value: Isolation) {
        count += value.value
    }
    func publishOrdered() -> Int {
        applicationEventPublisher.publishEvent(Ordered(status: "paid", total: 12))
        return 1
    }
    func publishOther() -> Int {
        applicationEventPublisher.publishEvent(Other(status: "paid", total: 12))
        return 1
    }
    @Order(-1)
    @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
    func ordinal(_ value: Ordered) -> Echo {
        log += "1"
        return Echo(amount: 7)
    }
    @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
    func otherOrdinal(_ value: Other) -> Echo {
        log += "1"
        return Echo(amount: 7)
    }
    @OnEvent
    func second(_ value: Ordered) {
        log += "2"
    }
    @OnEvent
    func failing(_ value: Ordered) throws {
        throw NSError(domain: "fixture", code: 1)
    }
    @OnEvent({"condition":"value.total >= 100"})
    func skipped(_ value: Ordered) {
        log += "S"
    }
    @Order(1)
    @InBackground
    @OnEvent
    func deferredListener(_ value: Ordered) -> Echo {
        log += "A"
        return Echo(amount: 1000)
    }
    @Order(2)
    @OnEvent
    func third(_ value: Ordered) {
        log += "3"
    }
    @OnEvent
    func onEcho(_ value: Echo) {
        log += "E"
        count += value.amount
    }
}
