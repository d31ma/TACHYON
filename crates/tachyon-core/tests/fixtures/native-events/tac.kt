class OwnerProbeService {
    var count: Int = 0
    var log: String = ""
    val deaths: Int = tacInt(TacStore.session("eventDeaths", 0))
    val applicationEventPublisher: ApplicationEventPublisher
    @OnStart
    fun start() {
        count = 1
        send()
    }
    @OnStop
    fun stop() {
        TacStore.setSession("eventDeaths", deaths + 1)
    }
    fun send(): Int {
        applicationEventPublisher.publishEvent(models.Pulse(5), true)
        return 5
    }
    @OnEvent
    fun receive(value: models.Pulse) {
        count += value.amount
        relay()
    }
    fun relay(): Int {
        applicationEventPublisher.publishEvent(models.Relay(3))
        return 3
    }
    @OnEvent
    fun receiveRelay(value: models.Relay) {
        count += value.amount
    }
    fun cycle(): Int {
        applicationEventPublisher.publishEvent(models.Cycle(1))
        return 1
    }
    @OnEvent
    fun receiveCycle(value: models.Cycle) {
        count += value.amount
        cycle()
    }
    fun isolated(): Map<String, Int> {
        applicationEventPublisher.publishEvent(models.Isolation(1))
        return mapOf("value" to 1)
    }
    @OnEvent
    fun aMutation(value: models.Isolation) {
        val copy = value.copy(value = 99)
        count += copy.value - 99
    }
    @OnEvent
    fun bOriginal(value: models.Isolation) {
        count += value.value
    }
    fun publishOrdered(): Int {
        applicationEventPublisher.publishEvent(models.Ordered("paid", 12))
        return 1
    }
    fun publishOther(): Int {
        applicationEventPublisher.publishEvent(models.Other("paid", 12))
        return 1
    }
    @Order(-1)
    @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
    fun ordinal(value: models.Ordered): models.Echo {
        log += "1"
        return models.Echo(7)
    }
    @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
    fun otherOrdinal(value: models.Other): models.Echo {
        log += "1"
        return models.Echo(7)
    }
    @OnEvent
    fun second(value: models.Ordered) {
        log += "2"
    }
    @OnEvent
    fun failing(value: models.Ordered) {
        throw IllegalStateException("fixture")
    }
    @OnEvent({"condition":"value.total >= 100"})
    fun skipped(value: models.Ordered) {
        log += "S"
    }
    @Order(1)
    @InBackground
    @OnEvent
    fun deferredListener(value: models.Ordered): models.Echo {
        log += "A"
        return models.Echo(1000)
    }
    @Order(2)
    @OnEvent
    fun third(value: models.Ordered) {
        log += "3"
    }
    @OnEvent
    fun onEcho(value: models.Echo) {
        log += "E"
        count += value.amount
    }
}
