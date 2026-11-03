import models.Greeting

@RequestRoute("/greeting")
@Public
class GreetingController {
  @Needs
  @Named("LoudGreeterService")
  val loud: GreeterService

  @Needs
  val plain: GreeterService

  @GetRoute
  fun getGreeting(): Greeting = Greeting(loud.greet(), plain.greet())
}
