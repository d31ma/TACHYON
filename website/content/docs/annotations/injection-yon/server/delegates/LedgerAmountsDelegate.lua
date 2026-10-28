-- @Delegate
-- @Lifetime("singleton")
---@class LedgerAmountsDelegate
LedgerAmountsDelegate = {}

---@return integer
function LedgerAmountsDelegate:subtotal()
  return 40
end

---@return integer
function LedgerAmountsDelegate:surcharge()
  return 2
end
