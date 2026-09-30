@Test(GreetingTestSetup, {"partition":"tests"})
class GreetingControllerTest {
  Future<void> testGetGreetingSaysHello() async {
    final controller = GreetingController();
    final greeting = controller.getGreeting();
    Assert.equals('Hello from Yon', greeting.message);
  }
}
