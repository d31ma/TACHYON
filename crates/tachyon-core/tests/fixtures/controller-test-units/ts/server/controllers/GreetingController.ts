@RequestRoute("/api/greeting")
@Public
class GreetingController {
  @Needs
  greetingService: GreetingService;

  @GetRoute
  getGreeting() { return { message: this.greetingService.message() }; }
}
