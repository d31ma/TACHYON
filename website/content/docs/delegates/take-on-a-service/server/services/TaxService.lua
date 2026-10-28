-- @Service
---@class TaxService
TaxService = {}

---@param region string
---@return integer
function TaxService:rate(region)
  return 2
end

---@param count integer
---@return integer
function TaxService:total(count)
  return count * 10
end
