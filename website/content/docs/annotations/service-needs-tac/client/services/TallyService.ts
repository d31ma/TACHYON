@Service
export default class TallyService {
  @Needs
  counter!: CounterService;
  total = 0;

  bump(): void {
    this.total = this.counter.increment();
  }
}
