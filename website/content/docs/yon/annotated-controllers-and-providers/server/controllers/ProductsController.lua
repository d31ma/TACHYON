-- @RequestRoute("/products")
-- @Public
---@class ProductsController
ProductsController = {
  -- @Needs
  -- @Named("ProductsService")
  ---@type ProductsService
  catalog = nil,
}

---@return Product[]
-- @GetRoute
function ProductsController:get_products()
  return self.catalog:list()
end

---@param id integer
---@return Product
-- @GetRoute("/{id}")
function ProductsController:get_product(
  -- @FromPath
  id
)
  return self.catalog:find(id)
end
