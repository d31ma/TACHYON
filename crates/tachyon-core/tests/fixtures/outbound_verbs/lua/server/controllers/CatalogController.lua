-- @RequestRoute("/catalog")
-- @Public
---@class CatalogController
CatalogController = {
  -- @Needs
  ---@type CatalogClient
  catalog_client = nil,
}

---@return table
-- @GetRoute
function CatalogController:get_catalog()
  local catalog = self.catalog_client
  return {
    found = catalog:find("anvil", "full", "acme"),
    saved = catalog:save({ sku = "anvil", level = 3 }),
    replaced = catalog:replace(5, { sku = "anvil", level = 4 }),
    renamed = catalog:rename(6, { sku = "hammer", level = 1 }),
    removed = catalog:remove(7),
  }
end
