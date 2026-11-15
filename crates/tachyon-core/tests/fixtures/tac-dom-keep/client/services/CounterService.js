@Service
export default class CounterService {
  /** @type {number} */
  count = 0

  /** @returns {void} */
  bump() {
    this.count += 1
  }
}
