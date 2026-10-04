-- @Service
---@class CatalogService
CatalogService = {
  -- @Needs
  ---@type ProductConverter
  product_converter = nil,
  ---@type string
  title = "Not mapped.",
}

---@return nil
function CatalogService:load()
  local product = {detail = {id = 7}, name = "Anvil", extra = "dropped"}
  local summary = self.product_converter:to_summary(product)
  self.title = summary.kind .. " " .. summary.productId .. ": " .. summary.title
end
