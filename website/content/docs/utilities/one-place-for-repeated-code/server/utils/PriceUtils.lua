---@class PriceUtils
PriceUtils = {}

---@param cents integer
---@return string
function PriceUtils.label(cents)
  return "$" .. PriceUtils._dollars(cents)
end

---@param cents integer
---@return string
function PriceUtils._dollars(cents)
  return string.format("%.2f", cents / 100)
end
