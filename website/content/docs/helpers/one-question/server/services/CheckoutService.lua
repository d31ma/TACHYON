-- @Service
---@class CheckoutService
CheckoutService = {
  -- @Needs
  ---@type TaxHelper
  helper = nil,
}

---@return integer
function CheckoutService:total()
  local subtotal = self:subtotal()
  return self.helper:add_tax(subtotal)
end

---@private
---@return integer
function CheckoutService:subtotal()
  return 40
end
