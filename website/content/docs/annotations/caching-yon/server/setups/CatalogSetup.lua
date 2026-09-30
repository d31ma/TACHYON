-- @Setup
---@class CatalogSetup
CatalogSetup = {}

---@param execution table
---@return table
function CatalogSetup.configure(execution)
  local cache = Behaviors.construct(MemoryCacheHelper, { entries = {} })
  return { caches = { catalog = cache }, authorize = MemoryCacheHelper.allow }
end
