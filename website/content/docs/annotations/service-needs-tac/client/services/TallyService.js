@Service
export default class TallyService {
  /** @type {CounterService} */
  @Needs
  counter;
  total = 0;

  bump() {
    this.total = this.counter.increment();
  }
}
