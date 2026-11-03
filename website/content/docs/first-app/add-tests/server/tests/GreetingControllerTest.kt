@Test(GreetingTestSetup::class, {"partition":"tests"})
class GreetingControllerTest {
    fun testGetGreetingSaysHello() {
        val controller = GreetingController()
        val greeting = controller.getGreeting()
        Assert.equals("Hello from Yon", greeting.message)
    }
}
