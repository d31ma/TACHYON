@Test(StatusTestSetup, {"partition":"tests"})
class StatusControllerTest {
  Future<void> testGetStatusSaysOk() async {
    if (StatusController().getStatus() != 'ok') throw StateError('expected ok');
  }
}
