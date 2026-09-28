@Service
export default class BasketService {
  count = 6;
  /** @returns {number} */
  doubled() { return this.count * 2; }
  @OnStart
  initialize() { document.title = 'Basket'; }
}