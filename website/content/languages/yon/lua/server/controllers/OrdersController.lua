-- @RequestRoute("/orders")
-- @Public
---@class OrdersController
OrdersController = {
  -- @Needs
  ---@type OrderService
  order_service = nil,
}

---@return OrderList
-- @GetRoute
function OrdersController:get_orders()
  return { orders = self.order_service:list() }
end
