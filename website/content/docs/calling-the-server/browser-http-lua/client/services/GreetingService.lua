-- @Service
---@class GreetingService
GreetingService = {
  -- @Needs
  ---@type GreetingClient
  greeting_client = nil,
  -- @Needs
  ---@type OverrideClient
  override_client = nil,
  ---@type string
  message = "",
  ---@type string
  custom = "",
  ---@type integer
  loads = 0,
  ---@type integer
  clears = 0,
}

---@return nil
function GreetingService:ask()
  self.loads = self.loads + 1
  self.message = self.greeting_client:read().message
end

---@return nil
function GreetingService:clear()
  self.greeting_client:clear()
  self.clears = self.clears + 1
end

---@return nil
function GreetingService:ask_custom()
  self.custom = self.override_client:read().message
end
