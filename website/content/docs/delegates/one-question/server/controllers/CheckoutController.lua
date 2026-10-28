-- @RequestRoute("/checkout")
-- @Public
---@class CheckoutController
CheckoutController = {
  -- @Needs
  ---@type CheckoutService
  service = nil,
}

---@return Receipt
-- @GetRoute
function CheckoutController:get_checkout()
  local total = self.service:total()
  return { total = total }
end
