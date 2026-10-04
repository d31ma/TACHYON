-- @RequestRoute("/products")
-- @Public
---@class ProductController
ProductController = {
  ACTIVE_STATUS = "active",
  -- @Needs
  ---@type ProductStore
  product_store = nil,
}

---@return ProductReport
-- @GetRoute
function ProductController:get_product()
  return {
    active = self.product_store:find_by_status(self.ACTIVE_STATUS),
    count = self.product_store:count_by_status(self.ACTIVE_STATUS),
    others = self.product_store:excluding(self.ACTIVE_STATUS, { size = 5 }),
  }
end
