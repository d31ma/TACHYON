-- @Service
---@class ReleaseService
ReleaseService = {
  ---@type string
  release = "",
  ---@type ApplicationEventPublisher
  application_event_publisher = nil,
}

---@return nil
function ReleaseService:announce()
  local changed = setmetatable({ release = "TACHYON" }, ReleaseChanged)
  self.application_event_publisher:publish_event(changed, true)
end

---@param changed ReleaseChanged
---@return nil
-- @OnEvent({"replay":true})
function ReleaseService:receive(changed)
  self.release = changed.release
end
