@Service
export default class CounterService {
  count = 0

  increment(): void {
    this.count += 1
  }
}
