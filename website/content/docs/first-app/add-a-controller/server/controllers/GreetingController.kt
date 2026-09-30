import models.Greeting

@RequestRoute("/api/greeting")
@Public
class GreetingController {
    @GetRoute
    fun getGreeting(): Greeting = Greeting("Hello from Yon")
}
