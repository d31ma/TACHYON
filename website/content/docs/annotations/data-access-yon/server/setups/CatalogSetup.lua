-- @Setup
---@class CatalogSetup
CatalogSetup = {}

---@param execution table
---@return table
function CatalogSetup.configure(execution)
  return { entities = { catalog = CatalogRowsDelegate }, authorizeEntity = CatalogRowsDelegate.product_entity }
end
