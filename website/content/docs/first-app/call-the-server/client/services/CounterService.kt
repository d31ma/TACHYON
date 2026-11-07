@Service
class CounterService {
  @Needs
  private val greetingClient: GreetingClient
  var count: Int = 0
  var message: String = ""

  fun increment() {
    count += 1
  }

  suspend fun askServer() {
    message = greetingClient.getGreeting().message
  }
}
