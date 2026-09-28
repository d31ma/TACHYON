import models.PlacedOrder

class CheckoutService {
    var placed: Int = 0
    val applicationEventPublisher: ApplicationEventPublisher

    @OnStart
    fun start() { place() }

    fun place() {
        placed += 1
        applicationEventPublisher.publishEvent(PlacedOrder("paid", 25), true)
    }

    @OnStop
    fun stop() { placed = 0 }
}
