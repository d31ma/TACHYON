---@class CheckoutService
CheckoutService = {
  ---@type integer
  placed = 0,
  ---@type ApplicationEventPublisher
  application_event_publisher = nil,
}

---@return nil
-- @OnStart
function CheckoutService:start()
  self:place()
end

---@return nil
function CheckoutService:place()
  self.placed = self.placed + 1
  local order = setmetatable({ status = "paid", total = 25 }, PlacedOrder)
  self.application_event_publisher:publish_event(order, true)
end

---@return nil
-- @OnStop
function CheckoutService:stop()
  self.placed = 0
end
