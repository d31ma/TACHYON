@Service
export default class CounterService {
  count = 0;

  /** @returns {number} */
  increment() {
    this.count += 1;
    return this.count;
  }
}
