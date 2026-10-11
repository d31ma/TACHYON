#[Test(HelloTestSetup, {"partition":"tests"})]
class CounterServiceTest
  #[Fake]
  attr_accessor :greeting_client

  # @return [void]
  def test_increment_adds_one
    counter = construct(CounterService)
    counter.increment
    Assert.equals(1, counter.count)
  end

  # @return [void]
  def test_ask_server_saves_the_message
    counter = construct(CounterService)
    counter.ask_server
    Assert.equals("Hi", counter.message)
  end
end
