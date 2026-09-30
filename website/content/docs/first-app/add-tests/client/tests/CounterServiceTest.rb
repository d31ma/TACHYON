#[Test(HelloTestSetup, {"partition":"tests"})]
class CounterServiceTest
  # @return [void]
  def test_increment_adds_one
    counter = CounterService.new
    counter.increment
    Assert.equals(1, counter.count)
  end
end
