-- @RequestRoute("/billing")
-- @Public
---@class BillingController
BillingController = {
  -- @Needs
  ---@type BillingService
  billing_service = nil,
}

---@return Bill
-- @GetRoute
function BillingController:get_billing()
  return { total = self.billing_service:total() }
end
