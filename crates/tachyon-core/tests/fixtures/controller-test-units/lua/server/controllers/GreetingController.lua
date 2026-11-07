-- @RequestRoute("/api/greeting")
-- @Public
---@class GreetingController
GreetingController = {
  -- @Needs
  ---@type GreetingService
  greeting_service = nil,
}

---@return table
-- @GetRoute
function GreetingController:get_greeting()
  return { message = self.greeting_service:message() }
end
