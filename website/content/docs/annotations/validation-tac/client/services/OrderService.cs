[Service]
public class OrderService {
    public string status = "ready";
    public BehaviorJSON accept([Check(typeof(OrderInput))] BehaviorJSON order) { return order; }
}
