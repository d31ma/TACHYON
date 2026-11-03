-- @Service
---@class PriceService
PriceService = {
  CURRENCY = "EUR",
}

---@param amount integer
---@param count integer
---@return Price
function PriceService:price_for(amount, count)
  return { amount = amount * count, currency = PriceService.CURRENCY }
end
