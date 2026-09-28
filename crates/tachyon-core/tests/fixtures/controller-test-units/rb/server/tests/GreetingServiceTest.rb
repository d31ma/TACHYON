#[Test(GreetingTestSetup, {"partition":"tests"})]
class GreetingServiceTest
  def test_message_says_hello
    raise "expected Hello" unless construct(GreetingService).message == "Hello"
  end
end
