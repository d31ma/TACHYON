@Service
final class CartService {
    var total: Int = 0
    let applicationEventPublisher: ApplicationEventPublisher

    func add() {
        total += 1
        applicationEventPublisher.publishEvent(CartTotal(total: total), true)
    }
}
