-- @Setup
---@class CatalogSetup
CatalogSetup = {}

---@param execution table
---@return table
function CatalogSetup.configure(execution)
  local cache = Behaviors.construct(MemoryCacheDelegate, { entries = {} })
  return { caches = { catalog = cache }, authorize = MemoryCacheDelegate.allow }
end
