[Service]
class OrderService {
    public BehaviorJSON accept([Check(typeof(OrderInput))] BehaviorJSON order) { return order; }
}
