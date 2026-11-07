@Delegate
export default class CartPricesDelegate {
  subtotal(): number { return 40; }
  surcharge(): number { return 2; }
  count(): number { return 3; }
}
