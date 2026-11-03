-- @RequestRoute("/ledger")
-- @Public
---@class LedgerController
LedgerController = {
  -- @Needs
  ---@type LedgerClient
  ledger_client = nil,
}

---@return Balance
-- @GetRoute
function LedgerController:get_ledger()
  return {
    balance = self.ledger_client:balance("ACME-7"),
    forecast = self.ledger_client:forecast(1),
  }
end
