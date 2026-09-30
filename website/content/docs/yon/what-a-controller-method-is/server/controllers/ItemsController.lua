-- @RequestRoute("/items")
-- @Public
---@class ItemsController
ItemsController = {}

---@return ItemList
-- @GetRoute
function ItemsController:get_items()
  return { items = {} }
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
