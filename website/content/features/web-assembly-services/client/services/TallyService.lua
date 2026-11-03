-- @Service
---@class TallyService
TallyService = {
  ---@type integer
  count = 0,
}

---@param amount integer
---@return integer
function TallyService:add(amount)
  self.count = self.count + amount
  return self.count
end

---@param unit string
---@return string
function TallyService:summary(unit)
  return self.count .. " " .. unit
end
