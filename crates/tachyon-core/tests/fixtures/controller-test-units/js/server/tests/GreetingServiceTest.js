@Test(GreetingTestSetup, {"partition":"tests"})
export default class GreetingServiceTest {
  async testMessageSaysHello() {
    const service = this.construct(GreetingService);
    Assert.equals('Hello', service.message());
  }
}
