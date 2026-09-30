/** @import { CartTotal } from '/models/CartTotal.ts' */
@Service
export default class CartService {
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;
  total = 0;
  add() {
    this.total += 1;
    /** @type {CartTotal} */
    const changed = { total: this.total };
    this.applicationEventPublisher.publishEvent(changed, true);
  }
}
