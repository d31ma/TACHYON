@Service
class CounterService {
  @Needs
  final GreetingClient greetingClient;
  int count = 0;
  String message = '';

  void increment() {
    count += 1;
  }

  Future<void> askServer() async {
    message = (await greetingClient.getGreeting()).message;
  }
}
