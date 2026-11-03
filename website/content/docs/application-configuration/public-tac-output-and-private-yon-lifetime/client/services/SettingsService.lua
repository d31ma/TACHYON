-- @Service
---@class SettingsService
SettingsService = {
  -- @Needs
  ---@type ApplicationClient
  application_client = nil,
  ---@type string
  api = "",
}

---@return nil
function SettingsService:load()
  local application = self.application_client:load()
  self.api = Behaviors.values(PublicSettings, application).api
end
