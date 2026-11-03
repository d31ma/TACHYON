-- @Service
---@class ProductService
ProductService = {}

---@param input table
---@return table
function ProductService:accept(
  -- @Check(ProductInput)
  input
)
  return input
end
