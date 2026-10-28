-- @RequestRoute("/ledger")
-- @Public
---@class LedgerController
LedgerController = {
  -- @Needs
  ---@type LedgerService
  ledger_service = nil,
}

---@return LedgerReport
-- @GetRoute
function LedgerController:get_ledger()
  return { settled = self.ledger_service:settle("open"), commands = LedgerAdapterDelegate.commands }
end
