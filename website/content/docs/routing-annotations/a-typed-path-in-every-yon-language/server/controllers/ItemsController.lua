-- @RequestRoute("/items")
-- @Public
---@class ItemsController
ItemsController = {}

---@param id integer
---@return Item
-- @GetRoute("/{id}")
function ItemsController:get_item(
  -- @FromPath
  id
)
  return { id = id }
end
