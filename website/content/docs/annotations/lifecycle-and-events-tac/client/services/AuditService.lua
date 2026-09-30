---@class AuditService
AuditService = {
  ---@type integer
  settled = 0,
  ---@type integer
  _records = 0,
  ---@type integer
  _audited = 0,
}

---@param order PlacedOrder
---@return nil
-- @OnEvent({"id":"audit-log","replay":true})
-- @Order(-10)
function AuditService:record(order)
  self._records = self._records + 1
end

---@param order PlacedOrder
---@return SettledOrder
-- @OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
-- @Order(10)
function AuditService:settle(order)
  return setmetatable({ total = order.total }, SettledOrder)
end

---@param order SettledOrder
---@return nil
-- @OnEvent
function AuditService:save_settled(order)
  self.settled = order.total
end

---@param order SettledOrder
---@return nil
-- @OnEvent
-- @InBackground
function AuditService:audit(order)
  self._audited = order.total
end
