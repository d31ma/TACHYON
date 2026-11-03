-- @Setup
---@class CatalogSetup
CatalogSetup = {}

---@param execution table
---@return table
function CatalogSetup.configure(execution)
  return { http = { catalog = CatalogSetup } }
end

---@param request table
---@param execution table
---@return string[]
function CatalogSetup:exchange(request, execution)
  return { "anvil", "rope" }
end
