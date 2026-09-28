[Test(typeof(StatusTestSetup), {"partition":"tests"})]
sealed class StatusControllerTest {
    void testGetStatusSaysOk() {
        if (new StatusController().GetStatus() != "ok") throw new System.Exception("expected ok");
    }
}
