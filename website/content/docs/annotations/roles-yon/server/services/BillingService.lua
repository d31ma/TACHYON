-- @Service
---@class BillingService
BillingService = {
  -- @Needs
  ---@type BillingHelper
  billing_helper = nil,
}

---@return integer
function BillingService:total()
  local subtotal = 40
  return self.billing_helper:settle(subtotal)
end
