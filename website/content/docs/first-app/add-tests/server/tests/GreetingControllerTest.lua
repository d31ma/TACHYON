-- @Test(GreetingTestSetup, {"partition":"tests"})
---@class GreetingControllerTest
GreetingControllerTest = {}

function GreetingControllerTest:test_get_greeting_says_hello()
  local greeting = GreetingController:get_greeting()
  Assert.equals("Hello from Yon", greeting.message)
end
