@Service
final class OrderService {
    var status: String = "ready"
    func accept(@Check(OrderInput) _ order: BehaviorJSON) throws -> BehaviorJSON { return order }
}
