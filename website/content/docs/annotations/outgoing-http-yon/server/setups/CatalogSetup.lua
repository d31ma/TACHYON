-- @Setup
---@class CatalogSetup
CatalogSetup = {}

---@param execution table
---@return table
function CatalogSetup.configure(execution)
  return { http = { catalog = CatalogTransportHelper } }
end
