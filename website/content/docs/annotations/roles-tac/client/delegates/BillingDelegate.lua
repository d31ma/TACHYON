-- @Delegate
---@class BillingDelegate
BillingDelegate = {}

---@param subtotal integer
---@return integer
function BillingDelegate:settle(subtotal)
  return subtotal + 2
end
