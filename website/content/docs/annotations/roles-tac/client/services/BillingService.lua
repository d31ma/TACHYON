-- @Service
---@class BillingService
BillingService = {
  -- @Needs
  ---@type BillingDelegate
  billing_delegate = nil,
  ---@type integer
  total = 0,
  ---@type integer
  subtotal = 40,
}

---@return nil
-- @OnStart
function BillingService:open()
  self.total = self.billing_delegate:settle(self.subtotal)
end
