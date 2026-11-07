-- @Service
---@class PublishService
PublishService = {
  ---@type ApplicationEventPublisher
  application_event_publisher = nil,
}

---@return nil
-- @OnStart
function PublishService:initialize()
  self.application_event_publisher:publish_event(setmetatable({ value = 7 }, ProbeReady))
end
