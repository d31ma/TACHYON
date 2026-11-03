-- @RequestRoute("/products")
-- @Public
---@class ProductsController
ProductsController = {}

---@return table
-- @GetRoute
function ProductsController:get_products()
  return { 200, { ["content-type"] = "text/html; charset=utf-8" }, "<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>" }
end
