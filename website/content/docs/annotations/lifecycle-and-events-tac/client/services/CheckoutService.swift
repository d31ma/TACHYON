final class CheckoutService {
    var placed: Int = 0
    let applicationEventPublisher: ApplicationEventPublisher

    @OnStart
    func start() { place() }

    func place() {
        placed += 1
        applicationEventPublisher.publishEvent(PlacedOrder(status: "paid", total: 25), true)
    }

    @OnStop
    func stop() { placed = 0 }
}
