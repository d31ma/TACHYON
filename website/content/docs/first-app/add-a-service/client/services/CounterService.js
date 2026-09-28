@Service
export default class CounterService {
  count = 0

  increment() {
    this.count += 1
  }
}
