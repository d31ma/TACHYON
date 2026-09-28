@Test(StatusTestSetup::class, {"partition":"tests"})
class StatusControllerTest {
    fun testGetStatusSaysOk() {
        if (StatusController().getStatus() != "ok") throw IllegalStateException("expected ok")
    }
}
