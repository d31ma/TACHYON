export class OrderService {
  static place(item: string): { placed: string } {
    return { placed: item }
  }
}
