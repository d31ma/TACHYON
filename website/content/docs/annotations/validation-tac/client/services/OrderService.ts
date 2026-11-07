@Service
export default class OrderService {
  status: string = 'ready';
  accept(@Check order: OrderInput): OrderInput { return order; }
}
