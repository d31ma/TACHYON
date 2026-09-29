@Service
export default class OrderService {
  accept(@Check order: OrderInput): OrderInput {
    return order;
  }
}
