-- @Service
---@class CatalogService
CatalogService = {
  ---@type integer
  offset = 0,
}

---@param input table
---@return integer
function CatalogService:page(
  -- @PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})
  input
)
  return input.offset
end

---@return nil
-- @OnStart
function CatalogService:start()
  self.behaviors = Behaviors.create(ApplicationSetup, { partition = "public-catalog", signal = BehaviorSignal.new() })
  self.offset = self:page({ page = 1 })
end
