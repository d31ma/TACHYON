-- @Service
---@class CatalogService
CatalogService = {
  -- @Needs
  ---@type CatalogClient
  catalog_client = nil,
  ---@type integer
  count = 0,
}

---@return nil
function CatalogService:load()
  self.count = #self.catalog_client:all().items
end
