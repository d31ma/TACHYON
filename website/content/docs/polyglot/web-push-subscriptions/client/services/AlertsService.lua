-- @Service
---@class AlertsService
AlertsService = {
  ---@type string
  endpoint = "",
  ---@type string
  problem = "",
  ---@type string
  server_key = "BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U",
}

---@return nil
function AlertsService:turn_on()
  self:_keep_problem(Push.subscribe, Push, self.server_key)
end

---@return nil
function AlertsService:turn_off()
  self:_keep_problem(Push.unsubscribe, Push)
end

---@param subscribed PushSubscribed
---@return nil
-- @OnEvent
function AlertsService:subscribed(subscribed)
  self.endpoint = subscribed.endpoint
end

---@param unsubscribed PushUnsubscribed
---@return nil
-- @OnEvent
function AlertsService:unsubscribed(unsubscribed)
  self.endpoint = ""
end

---@param failed BrowserCallFailed
---@return nil
-- @OnEvent
function AlertsService:failed(failed)
  self.problem = failed.message
end

---@param call function
---@param api table
---@param key string|nil
---@return nil
function AlertsService:_keep_problem(call, api, key)
  local ok, refusal = pcall(call, api, key)
  if not ok then
    self.problem = refusal
  end
end
