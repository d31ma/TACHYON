-- @Test(GreetingTestSetup, {"partition":"tests"})
---@class GreetingControllerTest
GreetingControllerTest = {
  -- @Fake
  ---@type GreetingService
  greeting_service = nil,
}

function GreetingControllerTest:test_get_greeting_uses_the_fake()
  self.greeting_service = { message = function() return "Hi" end }
  Assert.equals("Hi", self:construct(GreetingController):get_greeting().message)
end
