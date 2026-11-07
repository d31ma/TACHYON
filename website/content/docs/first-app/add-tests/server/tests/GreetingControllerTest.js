@Test(GreetingTestSetup, {"partition":"tests"})
export default class GreetingControllerTest {
  async testGetGreetingSaysHello() {
    const controller = this.construct(GreetingController)
    const greeting = controller.getGreeting()
    Assert.equals('Hello from Yon', greeting.message)
  }
}
