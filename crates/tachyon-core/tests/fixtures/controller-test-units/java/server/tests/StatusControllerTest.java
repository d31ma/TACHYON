@Test(StatusTestSetup.class, {"partition":"tests"})
class StatusControllerTest {
    void testGetStatusSaysOk() {
        if (!new StatusController().getStatus().equals("ok")) throw new IllegalStateException("expected ok");
    }
}
