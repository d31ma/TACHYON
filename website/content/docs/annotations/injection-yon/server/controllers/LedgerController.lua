-- @RequestRoute("/ledger")
-- @Public
---@class LedgerController
LedgerController = {
  -- @Needs
  -- @Named("LedgerService")
  ---@type LedgerService
  service = nil,
  -- @Needs
  -- @Named("LedgerDelegate")
  ---@type LedgerDelegate
  settlement = nil,
}

---@return Ledger
-- @GetRoute
function LedgerController:get_ledger()
  local total = self.settlement:settle(self.service:subtotal())
  return { total = total }
end
