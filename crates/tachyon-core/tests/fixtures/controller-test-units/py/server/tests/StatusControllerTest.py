@Test(StatusTestSetup, {"partition":"tests"})
class StatusControllerTest:
    async def test_get_status_says_ok(self):
        Assert.equals("ok", StatusController().get_status())
