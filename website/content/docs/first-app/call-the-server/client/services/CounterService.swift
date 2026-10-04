@Service
final class CounterService {
  @Needs
  private let greetingClient: GreetingClient
  var count: Int = 0
  var message: String = ""

  func increment() {
    count += 1
  }

  func askServer() async throws {
    message = try await greetingClient.getGreeting().message
  }
}
