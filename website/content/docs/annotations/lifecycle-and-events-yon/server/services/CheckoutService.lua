-- @Service
---@class CheckoutService
CheckoutService = {
  ---@type ApplicationEventPublisher
  application_event_publisher = nil,
}

---@return nil
-- @OnStart
function CheckoutService:start()
  self.application_event_publisher:publish_event(setmetatable({ status = "paid", total = 25 }, PlacedOrder))
end

---@return nil
-- @OnStop
function CheckoutService:stop()
end
