-- @RequestRoute("/price")
-- @Public
---@class PriceController
PriceController = {
  -- @Needs
  ---@type PriceService
  price_service = nil,
}

---@return Price
-- @GetRoute
function PriceController:get_price()
  return self.price_service:price_for(20, 3)
end
