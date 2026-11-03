using Models;

[RequestRoute("/api/greeting")]
[Public]
class GreetingController {
    [GetRoute]
    public Greeting GetGreeting() {
        return new Greeting("Hello from Yon");
    }
}
