-- @Delegate
---@class BillingDelegate
BillingDelegate = {}

---@param subtotal integer
---@return integer
function BillingDelegate:settle(subtotal)
  local surcharge = 2
  return subtotal + surcharge
end
