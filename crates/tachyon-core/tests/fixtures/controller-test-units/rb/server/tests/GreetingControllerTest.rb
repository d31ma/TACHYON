#[Test(GreetingTestSetup, {"partition":"tests"})]
class GreetingControllerTest
  #[Fake]
  attr_accessor :greeting_service

  def test_get_greeting_uses_the_fake
    self.greeting_service = Struct.new(:message).new("Hi")
    Assert.equals("Hi", construct(GreetingController).get_greeting["message"])
  end
end
