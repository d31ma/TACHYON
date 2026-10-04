-- @Setup
---@class CatalogSetup
CatalogSetup = {}

---@param execution table
---@return table
function CatalogSetup.configure(execution)
  return { entities = { catalog = CatalogRowsHelper }, authorizeEntity = CatalogRowsHelper.product_entity }
end
