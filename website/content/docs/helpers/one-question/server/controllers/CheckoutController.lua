-- @RequestRoute("/checkout")
-- @Public
---@class CheckoutController
CheckoutController = {
  -- @Needs
  ---@type CheckoutService
  checkout_service = nil,
}

---@return Receipt
-- @GetRoute
function CheckoutController:get_checkout()
  local total = self.checkout_service:total()
  return { total = total }
end
