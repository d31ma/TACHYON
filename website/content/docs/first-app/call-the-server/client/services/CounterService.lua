-- @Service
---@class CounterService
CounterService = {
  -- @Needs
  ---@type GreetingClient
  greeting_client = nil,
  ---@type integer
  count = 0,
  ---@type string
  message = "",
}

function CounterService:increment()
  self.count = self.count + 1
end

---@return nil
function CounterService:ask_server()
  self.message = self.greeting_client:get_greeting().message
end
