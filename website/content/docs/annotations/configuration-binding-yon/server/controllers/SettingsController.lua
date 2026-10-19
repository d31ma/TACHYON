-- @RequestRoute("/settings")
-- @Public
---@class SettingsController
SettingsController = {}

---@return Settings
-- @GetRoute
function SettingsController:get_settings()
  local application = YonJson.decode(os.getenv("YON_APPLICATION_JSON"))
  local catalog = Behaviors.properties(CatalogSetup, application)
  local profile = Behaviors.values(Endpoint, application).profile
  local context = Behaviors.create(CatalogSetup, {
    partition = "public-catalog",
    signal = BehaviorSignal.new(),
    profiles = { profile },
    properties = application,
  })
  context:dispose()
  return { profile = profile, pageSize = catalog.page_size }
end
