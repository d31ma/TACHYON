-- @RequestRoute("/orders")
-- @Public
---@class OrdersController
OrdersController = {}

---@return Problem
-- @GetRoute
function OrdersController:get_order()
  error(setmetatable({ message = "order 42" }, OrderNotFound))
end
