-- @Service
---@class TallyService
TallyService = {
  ---@type integer
  count = 6,
  ---@type string
  runtime = "Lua, on the Lua web runtime",
}

---@return integer
function TallyService:doubled()
  return self.count * 2
end
