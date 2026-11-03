-- @Service
---@class SettingsService
SettingsService = {
  ---@type string
  title = "Catalog",
  ---@type integer
  page_size = 20,
}

---@return nil
-- @OnStart
function SettingsService:bind()
  local published = self:_public_manifest()
  local catalog = Behaviors.properties(CatalogSetup, published)
  local values = Behaviors.values(PublicValues, published)
  local context = Behaviors.create(CatalogSetup, {
    partition = "public-catalog",
    signal = BehaviorSignal.new(),
    profiles = { "public" },
    properties = published,
  })
  context:dispose()
  self.title = values.title
  self.page_size = catalog.page_size
end

---@return table
function SettingsService:_public_manifest()
  local catalog = { title = "Catalog", page = { size = 20 }, cache = { enabled = true } }
  return { tac = { catalog = catalog } }
end
