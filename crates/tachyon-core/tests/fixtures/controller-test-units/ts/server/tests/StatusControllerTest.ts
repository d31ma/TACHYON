@Test(GreetingTestSetup, {"partition":"tests"})
export default class StatusControllerTest {
  async testGetStatusSaysOk() {
    Assert.equals('ok', this.construct(StatusController).getStatus());
  }
}
