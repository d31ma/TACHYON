-- @RequestRoute("/products")
-- @Public
---@class ProductsController
ProductsController = {
  -- @Needs
  ---@type ProductService
  product_service = nil,
}

---@param product Product
---@return Product
-- @PostRoute
function ProductsController:post_product(
  -- @FromBody
  product
)
  self.product_service:accept({ name = product.name, level = product.level })
  return product
end
