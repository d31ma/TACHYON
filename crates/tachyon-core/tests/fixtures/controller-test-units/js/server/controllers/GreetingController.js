@RequestRoute("/api/greeting")
@Public
class GreetingController {
  @Needs
  greetingService;

  @GetRoute
  getGreeting() { return { message: this.greetingService.message() }; }
}
