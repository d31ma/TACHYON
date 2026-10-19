-- @Service
---@class OrdersService
OrdersService = {
  ---@type integer
  attempts = 0,
  ---@type integer
  first_working_attempt = 3,
}

---@return nil
-- @OnStart
function OrdersService:open()
  self.behaviors = Behaviors.create(OrdersSetup, { partition = "orders", signal = BehaviorSignal.new() })
end

---@param id string
---@return table
-- @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
function OrdersService:submit(id)
  self.attempts = self.attempts + 1
  self:fail_first_two_attempts()
  return self:report_of(id)
end

---@param id string
---@return table
-- @MaxAtOnce({"limit":2,"policy":"reject"})
function OrdersService:report(id)
  return self:report_of(id)
end

---@param id string
---@return table
function OrdersService:report_of(id)
  return { id = id, attempts = self.attempts }
end

---@return nil
function OrdersService:fail_first_two_attempts()
  if self.attempts < self.first_working_attempt then
    error("the checkout API is unavailable")
  end
end
