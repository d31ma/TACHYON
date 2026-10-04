-- @Service
---@class CartService
CartService = {
  ---@type ApplicationEventPublisher
  application_event_publisher = nil,
  ---@type integer
  total = 0,
}

---@return nil
function CartService:add()
  self.total = self.total + 1
  local changed = setmetatable({ total = self.total }, CartTotal)
  self.application_event_publisher:publish_event(changed, true)
end
