-- @Helper
---@class TaxHelper
TaxHelper = {
  -- @Needs
  ---@type TaxRateHelper
  rates = nil,
}

---@param subtotal integer
---@return integer
function TaxHelper:add_tax(subtotal)
  local tax = self.rates:rate()
  return subtotal + tax
end
