-- @Setup
---@class LedgerSetup
LedgerSetup = {}

---@param execution table
---@return table
function LedgerSetup.configure(execution)
  return { entities = { ledger = LedgerAdapterHelper }, authorizeEntity = LedgerSetup.authorize }
end

---@param command table
---@param execution table
---@return boolean
function LedgerSetup.authorize(command, execution)
  return true
end
