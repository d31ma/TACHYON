-- @Service
---@class StatusService
StatusService = {
  ---@type boolean
  online = true,
  ---@type string
  scheme = "light",
}

---@param changed ConnectivityChanged
---@return nil
-- @OnEvent({"replay":true})
function StatusService:connectivity_changed(changed)
  self.online = changed.online
end

---@param changed PreferencesChanged
---@return nil
-- @OnEvent({"replay":true})
function StatusService:preferences_changed(changed)
  self.scheme = changed.scheme
end
