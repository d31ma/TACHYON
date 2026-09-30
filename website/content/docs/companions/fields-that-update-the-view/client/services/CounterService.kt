@Service
class CounterService {
  var count: Int = 0
  @SessionState
  var visits: Int = 0
  @LocalState
  var theme: String = "light"

  fun increment() { count += 1 }
}
