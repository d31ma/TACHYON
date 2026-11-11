@Service
export default class TickerService {
  seconds = 0;

  @Every('1s')
  count() {
    this.seconds += 1;
  }
}
