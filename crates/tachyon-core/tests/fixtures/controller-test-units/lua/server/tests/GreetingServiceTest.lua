-- @Test(GreetingTestSetup, {"partition":"tests"})
---@class GreetingServiceTest
GreetingServiceTest = {}

function GreetingServiceTest:test_message_says_hello()
  Assert.equals("Hello", self:construct(GreetingService):message())
end
