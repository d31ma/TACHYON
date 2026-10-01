@Service
class GreetingService {
  @Needs
  final GreetingClient greetingClient;
  @Needs
  final OverrideClient overrideClient;
  String message = '';
  String custom = '';
  int loads = 0;
  int clears = 0;

  Future<void> ask() async {
    loads += 1;
    message = (await greetingClient.read()).message;
  }

  Future<void> clear() async {
    await greetingClient.clear();
    clears += 1;
  }

  Future<void> override() async {
    custom = (await overrideClient.read()).message;
  }
}
