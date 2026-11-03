-- @Service
---@class LedgerService
LedgerService = {
  -- @Needs
  ---@type LedgerAmountsDelegate
  ledger_amounts_delegate = nil,
}

---@return integer
function LedgerService:subtotal()
  return self.ledger_amounts_delegate:subtotal()
end
