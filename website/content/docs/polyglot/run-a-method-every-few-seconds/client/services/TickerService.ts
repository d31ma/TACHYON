@Service
export default class TickerService {
  seconds: number = 0;

  @Every('1s')
  count(): void {
    this.seconds += 1;
  }
}
