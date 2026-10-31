using Models;

[RequestRoute("/greeting")]
[Public]
class GreetingController {
  [Needs]
  [Named("LoudGreeterService")]
  GreeterService _loud;

  [Needs]
  GreeterService _plain;

  [GetRoute]
  public Greeting GetGreeting() { return new Greeting(_loud.greet(), _plain.greet()); }
}
