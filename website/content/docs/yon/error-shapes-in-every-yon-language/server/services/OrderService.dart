@Service
class OrderService {
  Object? accept(@Check(OrderInput) Object? order) {
    return order;
  }
}
