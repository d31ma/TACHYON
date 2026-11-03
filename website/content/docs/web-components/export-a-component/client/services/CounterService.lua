-- @Service
---@class CounterService
CounterService = {
  ---@type integer
  count = 0,
  ---@type ApplicationEventPublisher
  application_event_publisher = nil,
}

---@return nil
function CounterService:add()
  self.count = self.count + 1
  self.application_event_publisher:publish_event(setmetatable({ count = self.count }, CountChanged))
end
