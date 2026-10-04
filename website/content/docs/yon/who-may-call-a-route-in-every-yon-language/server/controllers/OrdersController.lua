-- @RequestRoute("/orders")
-- @SignedIn
---@class OrdersController
OrdersController = {}

---@return string[]
-- @GetRoute
-- @Public
function OrdersController:get_orders()
  return { "lamp", "desk" }
end

---@return string[]
-- @PostRoute
function OrdersController:post_order()
  return { "chair" }
end

---@return string[]
-- @DeleteRoute
-- @Allow(Admin)
function OrdersController:delete_orders()
  return {}
end
