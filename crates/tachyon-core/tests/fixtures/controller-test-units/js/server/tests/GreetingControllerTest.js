@Test(GreetingTestSetup, {"partition":"tests"})
export default class GreetingControllerTest {
  @Fake
  greetingService;

  async testGetGreetingUsesTheFake() {
    this.greetingService = { message: () => 'Hi' };
    const controller = this.construct(GreetingController);
    if (controller.getGreeting().message !== 'Hi') throw Error('expected Hi');
  }
}
