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
  return { name = "anvil", level = id }
end

---@param id integer
---@param item Item
---@return Item
-- @PatchRoute("/{id}")
function ItemsController:patch_item(
  -- @FromPath
  id,
  -- @FromBody
  item
)
  return { name = item.name, level = id }
end
