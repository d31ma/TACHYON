-- @RequestRoute("/orders")
-- @Allow(Admin)
---@class OrdersController
OrdersController = {
  -- @Needs
  ---@type OrderService
  order_service = nil,
}

---@return string[]
-- @GetRoute
function OrdersController:get_orders()
  error("the order store is down")
end

---@param order table
---@return table
-- @PostRoute
function OrdersController:post_order(
  -- @FromBody
  order
)
  return self.order_service:accept(order)
end
