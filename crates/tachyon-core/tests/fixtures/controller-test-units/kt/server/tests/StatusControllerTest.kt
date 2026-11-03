@Test(StatusTestSetup::class, {"partition":"tests"})
class StatusControllerTest {
    fun testGetStatusSaysOk() {
        Assert.equals("ok", StatusController().getStatus())
    }
}
