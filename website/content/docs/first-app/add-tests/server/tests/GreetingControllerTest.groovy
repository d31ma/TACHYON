@Test(GreetingTestSetup.class, {"partition":"tests"})
class GreetingControllerTest {
    void testGetGreetingSaysHello() {
        GreetingController controller = new GreetingController()
        Greeting greeting = controller.getGreeting()
        Assert.equals("Hello from Yon", greeting.message())
    }
}
