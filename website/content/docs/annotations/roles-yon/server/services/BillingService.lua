-- @Service
---@class BillingService
BillingService = {
  -- @Needs
  ---@type BillingDelegate
  billing_delegate = nil,
}

---@return integer
function BillingService:total()
  local subtotal = 40
  return self.billing_delegate:settle(subtotal)
end
