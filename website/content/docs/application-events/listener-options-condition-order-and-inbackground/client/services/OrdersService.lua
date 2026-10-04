-- @Service
---@class OrdersService
OrdersService = {
  ---@type ApplicationEventPublisher
  application_event_publisher = nil,
  ---@type string[]
  log = {},
  ---@type integer
  settled = 0,
  ---@type integer
  audited = 0,
}

---@param total integer
---@return nil
function OrdersService:place(total)
  local order = setmetatable({ status = "paid", total = total }, PlacedOrder)
  self.application_event_publisher:publish_event(order, false)
end

---@param order PlacedOrder
---@return nil
-- @OnEvent({"id":"orders-log"})
-- @Order(-10)
function OrdersService:record(order)
  self.log[#self.log + 1] = order.status
end

---@param order PlacedOrder
---@return SettledOrder
-- @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
function OrdersService:settle_large_order(order)
  self.settled = self.settled + 1
  return setmetatable({ total = order.total }, SettledOrder)
end

---@param settled SettledOrder
---@return nil
-- @OnEvent
-- @InBackground
function OrdersService:audit(settled)
  self.audited = settled.total
end
