import 'models/Greeting.dart';

@RequestRoute("/api/greeting")
@Public
class GreetingController {
  @GetRoute
  Greeting getGreeting() => const Greeting('Hello from Yon');
}
