@Service
class OrderService {
  accept(@Check order: OrderInput): OrderInput {
    return order;
  }
}
