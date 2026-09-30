-- @Service
---@class AuditService
AuditService = {
  ---@type string[]
  log = {},
  ---@type integer
  settled = 0,
}

---@param order PlacedOrder
---@return nil
-- @OnEvent({"id":"audit-log"})
-- @Order(-10)
function AuditService:record(order)
  self.log[#self.log + 1] = order.status
end

---@param order PlacedOrder
---@return SettledOrder
-- @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
-- @Order(10)
function AuditService:settle(order)
  return setmetatable({ total = order.total }, SettledOrder)
end

---@param settled SettledOrder
---@return nil
-- @OnEvent
function AuditService:save_settled(settled)
  self.settled = settled.total
end
