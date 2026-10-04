-- @Service
---@class LedgerService
LedgerService = {
  -- @Needs
  ---@type EntryStore
  entry_store = nil,
}

---@return nil
-- @OnStart
function LedgerService:open()
  self.behaviors = Behaviors.create(LedgerSetup, { partition = "ledger", signal = BehaviorSignal.new() })
end

---@param status string
---@return table
-- @AllOrNothing({"adapter":"ledger"})
function LedgerService:settle(status)
  return { open = self.entry_store:find_by_status(status), total = self.entry_store:count_by_status(status) }
end
