[Test(typeof(GreetingTestSetup), {"partition":"tests"})]
sealed class GreetingControllerTest {
    void testGetGreetingSaysHello() {
        var controller = new GreetingController();
        var greeting = controller.GetGreeting();
        Assert.equals("Hello from Yon", greeting.Message);
    }
}
