@Test(GreetingTestSetup, {"partition":"tests"})
class GreetingControllerTest:
    async def test_get_greeting_says_hello(self) -> None:
        controller = GreetingController()
        greeting = controller.get_greeting()
        Assert.equals('Hello from Yon', greeting.message)
