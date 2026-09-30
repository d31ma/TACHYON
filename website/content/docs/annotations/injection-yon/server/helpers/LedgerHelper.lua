-- @Helper
-- @Lifetime("transient")
---@class LedgerHelper
LedgerHelper = {
  -- @Needs
  ---@type LedgerAmountsHelper
  ledger_amounts_helper = nil,
}

---@param subtotal integer
---@return integer
function LedgerHelper:settle(subtotal)
  return subtotal + self.ledger_amounts_helper:surcharge()
end
