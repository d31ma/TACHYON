-- @Service
---@class TickerService
TickerService = {
  ---@type integer
  seconds = 0,
}

---@return nil
-- @Every("1s")
function TickerService:count()
  self.seconds = self.seconds + 1
end
