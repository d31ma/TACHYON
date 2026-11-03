-- @RequestRoute("/api/greeting")
-- @Public
---@class GreetingController
GreetingController = {}

---@return Greeting
-- @GetRoute
function GreetingController:get_greeting()
  return { message = "Hello from Yon" }
end
