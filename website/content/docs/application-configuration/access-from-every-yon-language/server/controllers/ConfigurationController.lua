-- @RequestRoute("/configuration")
-- @Public
---@class ConfigurationController
ConfigurationController = {}

---@return Configuration
-- @GetRoute
function ConfigurationController:get_configuration()
  local application = YonJson.decode(os.getenv("YON_APPLICATION_JSON"))
  local settings = Behaviors.properties(CatalogSettings, application)
  local endpoint = Behaviors.values(Endpoint, application)
  return { pageSize = settings.page_size, port = endpoint.port }
end
