from models import Greeting


@request_route("/api/greeting")
@public
class GreetingController:
    @get_route
    def get_greeting(self) -> Greeting:
        return Greeting("Hello from Yon")
