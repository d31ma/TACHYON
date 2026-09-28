@Test(StatusTestSetup, {"partition":"tests"})
class StatusControllerTest:
    async def test_get_status_says_ok(self):
        if StatusController().get_status() != "ok":
            raise Exception("expected ok")
