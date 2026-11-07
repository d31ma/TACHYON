[Test(typeof(StatusTestSetup), {"partition":"tests"})]
sealed class StatusControllerTest {
    void testGetStatusSaysOk() {
        Assert.equals("ok", new StatusController().GetStatus());
    }
}
