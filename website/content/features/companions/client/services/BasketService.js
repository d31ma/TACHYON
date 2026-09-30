@Service
export default class BasketService {
  count = 0;
  /** @returns {number} */
  doubled() { return this.count * 2; }
  @OnStart
  initialize() { this.count = 6; }
}
