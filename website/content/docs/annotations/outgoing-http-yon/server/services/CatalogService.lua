-- @Service
---@class CatalogService
CatalogService = {
  -- @Needs
  ---@type CatalogClient
  catalog_client = nil,
}

---@return table
function CatalogService:products()
  return self.catalog_client:all(1)
end
