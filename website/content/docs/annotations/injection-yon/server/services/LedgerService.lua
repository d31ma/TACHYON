-- @Service
---@class LedgerService
LedgerService = {
  -- @Needs
  ---@type LedgerAmountsHelper
  ledger_amounts_helper = nil,
}

---@return integer
function LedgerService:subtotal()
  return self.ledger_amounts_helper:subtotal()
end
