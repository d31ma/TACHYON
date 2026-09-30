-- @Service
---@class CatalogService
CatalogService = {
  ---@type string
  title = "Catalog",
}

---@param value integer
---@return integer
function CatalogService:double(value)
  return value * 2
end
