@Test(GreetingTestSetup, {"partition":"tests"})
export default class GreetingControllerTest {
  @Fake
  greetingService: GreetingService;

  async testGetGreetingUsesTheFake() {
    this.greetingService = { message: () => 'Hi' } as GreetingService;
    const controller = this.construct(GreetingController);
    if (controller.getGreeting().message !== 'Hi') throw Error('expected Hi');
  }
}
