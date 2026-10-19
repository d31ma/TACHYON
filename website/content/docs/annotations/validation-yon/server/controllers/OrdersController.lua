-- @RequestRoute("/orders")
-- @Public
---@class OrdersController
OrdersController = {
  -- @Needs
  ---@type OrderService
  order_service = nil,
}

---@param order table
---@return table
-- @PostRoute
function OrdersController:post_orders(
  -- @FromBody
  order
)
  return self.order_service:accept(order)
end
