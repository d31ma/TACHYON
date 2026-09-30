-- @Helper
---@class TaxHelper
TaxHelper = {
  -- @Needs
  ---@type TaxRateHelper
  tax_rate_helper = nil,
}

---@param subtotal integer
---@return integer
function TaxHelper:add_tax(subtotal)
  local tax = self.tax_rate_helper:rate()
  return subtotal + tax
end
