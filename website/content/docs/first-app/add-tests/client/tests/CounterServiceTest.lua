-- @Test(HelloTestSetup, {"partition":"tests"})
---@class CounterServiceTest
CounterServiceTest = {
  -- @Fake
  ---@type GreetingClient
  greeting_client = nil,
}

function CounterServiceTest:test_increment_adds_one()
  local counter = self:construct(CounterService)
  counter:increment()
  Assert.equals(1, counter.count)
end

function CounterServiceTest:test_ask_server_saves_the_message()
  local counter = self:construct(CounterService)
  counter:ask_server()
  Assert.equals("Hi", counter.message)
end
