-- @Helper
---@class CatalogRowsHelper
CatalogRowsHelper = {
  ---@type table[]
  ROWS = { { id = 1, status = "active" }, { id = 2, status = "retired" } },
}

---@param command table
---@param execution table
---@return any
function CatalogRowsHelper:execute(command, execution)
  local condition = command.query.where[1]
  local matching = {}
  for _, row in ipairs(self.ROWS) do
    if (row.status == condition.value) == (condition.operator == "eq") then
      matching[#matching + 1] = row
    end
  end
  if command.query.kind == "count" then
    return #matching
  end
  return matching
end

---@param command table
---@param execution table
---@return boolean
function CatalogRowsHelper.product_entity(command, execution)
  return command.entity == "ProductEntity"
end
