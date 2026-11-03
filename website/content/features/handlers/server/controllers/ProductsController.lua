-- @RequestRoute("/products")
-- @Public
---@class ProductsController
ProductsController = {}

---@return Product[]
-- @GetRoute
function ProductsController:get_products()
  return { { name = "anvil" } }
end

---@param product Product
---@return Product
-- @PostRoute
-- @ResponseStatus(201)
function ProductsController:post_product(
  -- @FromBody
  product
)
  return product
end
