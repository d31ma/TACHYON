-- @Service
---@class CounterService
CounterService = {
  ---@type integer
  count = 0,
  ---@type string
  theme = "light",
}

---@return nil
function CounterService:increment()
  self.count = self.count + 1
end
