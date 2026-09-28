@RequestRoute("/api/greeting")
@Public
class GreetingController {
  /** @type {GreetingService} */
  @Needs
  greetingService;

  /** @returns {{ message: string }} */
  @GetRoute
  getGreeting() { return { message: this.greetingService.message() }; }
}
