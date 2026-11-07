-- @RequestRoute("/catalog")
-- @Public
---@class CatalogController
CatalogController = {
  -- @Needs
  ---@type ProductConverter
  product_converter = nil,
}

---@return CatalogSummary
-- @GetRoute
function CatalogController:get_catalog()
  local product = { detail = { id = 7 }, name = "Anvil", extra = "dropped" }
  return { summary = self.product_converter:to_summary(product) }
end
