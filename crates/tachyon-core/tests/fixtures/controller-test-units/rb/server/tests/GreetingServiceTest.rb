#[Test(GreetingTestSetup, {"partition":"tests"})]
class GreetingServiceTest
  def test_message_says_hello
    Assert.equals("Hello", construct(GreetingService).message)
  end
end
