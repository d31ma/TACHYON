@Test(StatusTestSetup.class, {"partition":"tests"})
class StatusControllerTest {
    void testGetStatusSaysOk() {
        Assert.equals("ok", new StatusController().getStatus());
    }
}
