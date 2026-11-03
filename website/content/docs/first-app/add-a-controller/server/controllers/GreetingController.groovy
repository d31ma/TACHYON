import models.Greeting

@RequestRoute("/api/greeting")
@Public
class GreetingController {
    @GetRoute
    Greeting getGreeting() {
        return new Greeting("Hello from Yon")
    }
}
