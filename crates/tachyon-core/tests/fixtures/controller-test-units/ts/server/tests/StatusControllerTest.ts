@Test(GreetingTestSetup, {"partition":"tests"})
export default class StatusControllerTest {
  async testGetStatusSaysOk() {
    if (this.construct(StatusController).getStatus() !== 'ok') throw Error('expected ok');
  }
}
