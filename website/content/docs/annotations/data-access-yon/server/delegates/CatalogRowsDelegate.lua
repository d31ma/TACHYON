-- @Delegate
---@class CatalogRowsDelegate
CatalogRowsDelegate = {
  ---@type table[]
  ROWS = { { id = 1, status = "active" }, { id = 2, status = "retired" } },
}

---@param command table
---@param execution table
---@return any
function CatalogRowsDelegate:execute(command, execution)
  local query = command.query
  local condition = query.where[1]
  local matching = {}
  for _, row in ipairs(self.ROWS) do
    if (row[condition.field] == condition.value) == (condition.operator == "eq") then
      matching[#matching + 1] = row
    end
  end
  if query.kind == "count" then
    return #matching
  end
  if query.kind == "findOne" then
    return matching[1]
  end
  return matching
end

---@param command table
---@param execution table
---@return boolean
function CatalogRowsDelegate.product_entity(command, execution)
  return command.entity == "ProductEntity"
end
