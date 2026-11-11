-- @Schedule({"fixedDelaySeconds":300})
---@class Handler
Handler = {
  -- @Needs
  ---@type LedgerService
  ledger = nil,
}

---@return integer
function Handler:POST()
  return self.ledger:total()
end
