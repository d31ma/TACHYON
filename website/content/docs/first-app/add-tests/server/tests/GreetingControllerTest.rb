#[Test(GreetingTestSetup, {"partition":"tests"})]
class GreetingControllerTest
  # @return [void]
  def test_get_greeting_says_hello
    controller = construct(GreetingController)
    greeting = controller.get_greeting
    Assert.equals("Hello from Yon", greeting.message)
  end
end
