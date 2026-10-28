-- @Delegate
---@class TaxDelegate
TaxDelegate = {
  -- @Needs
  ---@type TaxRateDelegate
  rates = nil,
}

---@param subtotal integer
---@return integer
function TaxDelegate:add_tax(subtotal)
  local tax = self.rates:rate()
  return subtotal + tax
end
