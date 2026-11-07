import models.Greeting;

@RequestRoute("/greeting")
@Public
class GreetingController {
  @Needs
  @Named("LoudGreeterService")
  GreeterService loud;

  @Needs
  GreeterService plain;

  @GetRoute
  Greeting getGreeting() { return new Greeting(loud.greet(), plain.greet()); }
}
