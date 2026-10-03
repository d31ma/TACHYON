-- @Service
---@class PairService
PairService = {}

---@param first string?
---@param second integer
---@return string
function PairService:describe(first, second)
  return tostring(first) .. " " .. tostring(second)
end
