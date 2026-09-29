@Service
class OrderService {
    Object accept(@Check(OrderInput.class) Object order) {
        return order;
    }
}
