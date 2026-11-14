@Service
class OrderService {
    fun accept(@Check(OrderInput::class) order: Any?): Any? { return order }
}
