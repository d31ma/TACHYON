-- @Delegate
---@class CatalogTransportDelegate
CatalogTransportDelegate = {}

---@param request table
---@param execution table
---@return table
function CatalogTransportDelegate:exchange(request, execution)
  return { items = { "anvil" } }
end
