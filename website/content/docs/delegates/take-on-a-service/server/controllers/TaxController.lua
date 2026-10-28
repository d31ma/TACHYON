-- @RequestRoute("/tax")
-- @Public
---@class TaxController
TaxController = {
  -- @Needs
  ---@type TaxDelegate
  tax_delegate = nil,
}

---@return TaxAnswer
-- @GetRoute
function TaxController:get_tax()
  return {
    rate = self.tax_delegate:rate("north"),
    total = self.tax_delegate:total(3),
    extra = self.tax_delegate:total(3, 4),
  }
end
