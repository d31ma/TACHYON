-- @Delegate
-- @Lifetime("transient")
---@class LedgerDelegate
LedgerDelegate = {
  -- @Needs
  ---@type LedgerAmountsDelegate
  ledger_amounts_delegate = nil,
}

---@param subtotal integer
---@return integer
function LedgerDelegate:settle(subtotal)
  return subtotal + self.ledger_amounts_delegate:surcharge()
end
