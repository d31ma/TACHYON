@Test(GreetingTestSetup, {"partition":"tests"})
export default class GreetingServiceTest {
  async testMessageSaysHello() {
    const service = this.construct(GreetingService);
    if (service.message() !== 'Hello') throw Error('expected Hello');
  }
}
