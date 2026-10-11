@Service
export default class CounterService {
  count = 0;

  increment(): number {
    this.count += 1;
    return this.count;
  }
}
