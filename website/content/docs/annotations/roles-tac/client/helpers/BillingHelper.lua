-- @Helper
---@class BillingHelper
BillingHelper = {}

---@param subtotal integer
---@return integer
function BillingHelper:settle(subtotal)
  return subtotal + 2
end
