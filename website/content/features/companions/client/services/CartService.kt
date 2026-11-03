import models.CartTotal

@Service
class CartService {
    var total: Int = 0
    val applicationEventPublisher: ApplicationEventPublisher

    fun add() {
        total += 1
        applicationEventPublisher.publishEvent(CartTotal(total), true)
    }
}
