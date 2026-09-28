@Test(GreetingTestSetup, {"partition":"tests"})
export default class GreetingControllerTest {
  @Fake
  greetingService;

  async testGetGreetingUsesTheFake() {
    this.greetingService = { message: () => 'Hi' };
    const controller = this.construct(GreetingController);
    Assert.equals('Hi', controller.getGreeting().message);
  }
}
