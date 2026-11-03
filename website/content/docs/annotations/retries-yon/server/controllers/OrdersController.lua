-- @RequestRoute("/orders")
-- @Public
---@class OrdersController
OrdersController = {
  -- @Needs
  ---@type OrdersService
  orders_service = nil,
}

---@return OrderReport
-- @GetRoute
function OrdersController:get_orders()
  local id = "A-1"
  local submitted = self.orders_service:submit(id)
  local report = self.orders_service:report(id)
  return { submitted = submitted, report = report }
end
