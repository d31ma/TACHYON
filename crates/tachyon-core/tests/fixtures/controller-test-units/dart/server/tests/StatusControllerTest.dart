@Test(StatusTestSetup, {"partition":"tests"})
class StatusControllerTest {
  Future<void> testGetStatusSaysOk() async {
    Assert.equals('ok', StatusController().getStatus());
  }
}
