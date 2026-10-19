-- @Service
---@class BillingService
BillingService = {
  -- @Needs
  ---@type BillingHelper
  billing_helper = nil,
  ---@type integer
  total = 0,
  ---@type integer
  subtotal = 40,
}

---@return nil
-- @OnStart
function BillingService:open()
  self.total = self.billing_helper:settle(self.subtotal)
end
