@Test(GreetingTestSetup, {"partition":"tests"})
export default class GreetingControllerTest {
  @Fake
  greetingService: GreetingService;

  async testGetGreetingUsesTheFake() {
    this.greetingService = { message: () => 'Hi' } as GreetingService;
    const controller = this.construct(GreetingController);
    Assert.equals('Hi', controller.getGreeting().message);
  }
}
