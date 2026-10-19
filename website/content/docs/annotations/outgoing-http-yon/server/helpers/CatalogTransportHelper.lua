-- @Helper
---@class CatalogTransportHelper
CatalogTransportHelper = {}

---@param request table
---@param execution table
---@return table
function CatalogTransportHelper:exchange(request, execution)
  return { items = { "anvil" } }
end
