@Test(StatusTestSetup.class, {"partition":"tests"})
class StatusControllerTest {
    void testGetStatusSaysOk() {
        if (new StatusController().getStatus() != 'ok') {
            throw new IllegalStateException('expected ok')
        }
    }
}
