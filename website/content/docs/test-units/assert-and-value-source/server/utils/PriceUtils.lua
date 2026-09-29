---@class PriceUtils
PriceUtils = {}

---@param cents integer
---@return integer
function PriceUtils.with_tax(cents)
  if cents < 0 then
    error("A price is never negative.", 0)
  end
  return math.floor(cents * 1.2 + 0.5)
end
