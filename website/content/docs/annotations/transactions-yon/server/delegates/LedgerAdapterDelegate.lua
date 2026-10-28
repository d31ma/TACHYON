-- @Delegate
---@class LedgerAdapterDelegate
LedgerAdapterDelegate = {
  ---@type string[]
  commands = {},
  ---@type integer
  sequence = 0,
  ---@type table[]
  ROWS = { { id = 1, status = "open" }, { id = 2, status = "settled" } },
}

---@param command table
---@param execution table
---@return any
function LedgerAdapterDelegate:execute(command, execution)
  if command.operation == "transaction" then
    return self:_transaction(command)
  end
  local query = command.query
  self.commands[#self.commands + 1] = query.kind .. "@" .. (command.transaction or "none")
  local matching = {}
  for _, row in ipairs(self.ROWS) do
    if row.status == query.where[1].value then
      matching[#matching + 1] = row
    end
  end
  if query.kind == "count" then
    return #matching
  end
  return matching
end

---@param command table
---@return table
function LedgerAdapterDelegate:_transaction(command)
  local identifier = command.transaction
  self.commands[#self.commands + 1] = identifier and (command.action .. ":" .. identifier) or command.action
  if command.action ~= "begin" then
    return { ok = true }
  end
  self.sequence = self.sequence + 1
  return { transaction = "t" .. self.sequence }
end
