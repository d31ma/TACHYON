-- @Service
---@class ActivityService
ActivityService = {
  ---@type string
  state = "foreground",
}

---@param changed ApplicationStateChanged
---@return nil
-- @OnEvent({"replay":true})
function ActivityService:application_state_changed(changed)
  self.state = changed.state
end
