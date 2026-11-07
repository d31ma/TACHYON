-- @Service
---@class BasketService
BasketService = {
  ---@type integer
  _total = 1250,
}

---@return string
function BasketService:total_label()
  return PriceUtils.label(self._total)
end
