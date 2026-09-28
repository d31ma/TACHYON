import models.PlacedOrder

@Service
class CheckoutService {
  @Needs
  lateinit var applicationEventPublisher: ApplicationEventPublisher

  @OnStart
  fun start() {
    applicationEventPublisher.publishEvent(PlacedOrder("paid", 25))
  }

  @OnStop
  fun stop() {}
}
