-- @RequestRoute("/items")
-- @Public
---@class ItemsController
ItemsController = {}

---@param id string
---@return Item
-- @GetRoute("/{id}")
function ItemsController:get_item(
  -- @FromPath
  id
)
  return { name = id }
end

---@param item Item
---@return Item
-- @PostRoute
-- @ResponseStatus(201)
function ItemsController:post_item(
  -- @FromBody
  item
)
  return item
end
