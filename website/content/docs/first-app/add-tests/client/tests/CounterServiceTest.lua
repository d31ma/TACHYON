-- @Test(HelloTestSetup, {"partition":"tests"})
---@class CounterServiceTest
CounterServiceTest = {}

function CounterServiceTest:test_increment_adds_one()
  local counter = self:construct(CounterService)
  counter:increment()
  Assert.equals(1, counter.count)
end
