-- @Service
---@class OrdersService
OrdersService = {
  FIRST_WORKING_ATTEMPT = 3,
  ---@type integer
  _attempts = 0,
}

---@return nil
-- @OnStart
function OrdersService:open()
  self._attempts = 0
  self.behaviors = Behaviors.create(OrdersSetup, { partition = "orders", signal = BehaviorSignal.new() })
end

---@param id string
---@return table
-- @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
function OrdersService:submit(id)
  self._attempts = self._attempts + 1
  self:_fail_first_two_attempts()
  return self:_report_of(id)
end

---@param id string
---@return table
-- @MaxAtOnce({"limit":2,"policy":"reject"})
function OrdersService:report(id)
  return self:_report_of(id)
end

---@param id string
---@return table
function OrdersService:_report_of(id)
  return { id = id, attempts = self._attempts }
end

---@return nil
function OrdersService:_fail_first_two_attempts()
  if self._attempts < OrdersService.FIRST_WORKING_ATTEMPT then
    error("the payment gateway is unavailable")
  end
end
