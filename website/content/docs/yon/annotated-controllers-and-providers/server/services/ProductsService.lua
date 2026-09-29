-- @Service
-- @Lifetime("transient")
---@class ProductsService
ProductsService = {}

---@return Product[]
function ProductsService:list()
  return { self:find(1) }
end

---@param id integer
---@return Product
function ProductsService:find(id)
  return { id = id, name = "Anvil" }
end
