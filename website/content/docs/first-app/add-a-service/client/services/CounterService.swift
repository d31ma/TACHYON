@Service
final class CounterService {
  var count: Int = 0

  func increment() {
    count += 1
  }
}
