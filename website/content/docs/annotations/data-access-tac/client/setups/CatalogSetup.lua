-- @Setup
---@class CatalogSetup
CatalogSetup = {}

---@param execution table
---@return table
function CatalogSetup.configure(execution)
  local catalog = {
    execute = function(_, command, call)
      if command.query.kind == "count" then
        return 0
      end
      return {}
    end,
  }
  return {
    entities = { publicCatalog = catalog },
    authorizeEntity = function(command, call)
      return command.entity == "ProductEntity"
    end,
  }
end
