-- @Delegate
---@class TaxDelegate
TaxDelegate = {
  -- @Needs
  ---@type TaxService
  tax_service = nil,
}

---@param region string
---@return integer
function TaxDelegate:rate(region)
  return self.tax_service:rate(region) * 2
end

---@param count integer
---@param extra integer
---@return integer
function TaxDelegate:total(count, extra)
  return self.tax_service:total(count) + extra
end
