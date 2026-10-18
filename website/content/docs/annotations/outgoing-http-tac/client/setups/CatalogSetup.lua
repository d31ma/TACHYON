-- @Setup
---@class CatalogSetup
CatalogSetup = {}

---@param execution table
---@return table
function CatalogSetup.configure(execution)
  local catalog = {
    exchange = function(_, request, call)
      return { items = { "anvil" } }
    end,
  }
  return { http = { catalog = catalog } }
end
