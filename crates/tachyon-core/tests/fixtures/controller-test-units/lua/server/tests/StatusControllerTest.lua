-- @Test(GreetingTestSetup, {"partition":"tests"})
---@class StatusControllerTest
StatusControllerTest = {}

function StatusControllerTest:test_get_status_says_ok()
  Assert.equals("ok", self:construct(StatusController):get_status())
end
