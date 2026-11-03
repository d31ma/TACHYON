-- @RequestRoute("/catalog")
-- @Public
---@class CatalogController
CatalogController = {
  -- @Needs
  ---@type CatalogService
  catalog_service = nil,
}

---@return table
-- @GetRoute
function CatalogController:get_catalog()
  return self.catalog_service:products()
end
