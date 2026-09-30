@Service
final class CounterService {
  var count: Int = 0
  @SessionState
  var visits: Int = 0
  @LocalState
  var theme: String = "light"

  func increment() { count += 1 }
}
