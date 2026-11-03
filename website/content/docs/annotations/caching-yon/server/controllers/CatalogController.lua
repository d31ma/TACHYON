-- @RequestRoute("/catalog")
-- @Public
---@class CatalogController
CatalogController = {
  -- @Needs
  ---@type CatalogService
  catalog_service = nil,
}

---@return CacheReport
-- @GetRoute
function CatalogController:get_catalog()
  local sku = "anvil"
  local first = self.catalog_service:find(sku)
  local again = self.catalog_service:find(sku)
  self.catalog_service:write(sku, 99)
  local written = self.catalog_service:find(sku)
  self.catalog_service:invalidate(sku)
  local refreshed = self.catalog_service:find(sku)
  return { first = first, ["repeat"] = again, written = written, refreshed = refreshed }
end
