import models.OrderInput

@Service
class OrderService {
    var status: String = "ready"
    fun accept(@Check(OrderInput::class) order: Any?): Any? { return order }
}
