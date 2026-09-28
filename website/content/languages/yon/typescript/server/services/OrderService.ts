import type { Order } from '/models/Order';
@Service
export class OrderService {
  private repository = new OrderStore()

  list(): Order[] {
    return this.repository.all()
  }
}
