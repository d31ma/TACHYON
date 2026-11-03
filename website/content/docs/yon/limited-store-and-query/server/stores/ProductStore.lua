-- @Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})
---@class ProductStore
ProductStore = {}

---@param status string
---@return table[]
function ProductStore:find_by_status(status)
end

---@param status string
---@return integer
function ProductStore:count_by_status(status)
end

---@param status string
---@param page table
---@return table[]
-- @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
function ProductStore:excluding(
  status,
  -- @PageDefaults
  page
)
end
