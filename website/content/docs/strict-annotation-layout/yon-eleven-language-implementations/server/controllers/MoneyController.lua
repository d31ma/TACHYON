-- @RequestRoute("/money")
-- @Public
---@class MoneyController
MoneyController = {
  -- @Needs
  ---@type MoneyService
  money_service = nil,
}

---@return Money
-- @GetRoute
function MoneyController:get_money()
  return { amount = self.money_service:amount() }
end
