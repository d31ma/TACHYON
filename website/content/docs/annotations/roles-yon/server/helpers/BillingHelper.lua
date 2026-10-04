-- @Helper
---@class BillingHelper
BillingHelper = {}

---@param subtotal integer
---@return integer
function BillingHelper:settle(subtotal)
  local surcharge = 2
  return subtotal + surcharge
end
