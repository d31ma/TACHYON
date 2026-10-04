-- @Service
CatalogService = {
  -- @Needs
  ---@type ProductConverter
  product_converter = nil,
  ---@type string
  result = "",
}
---@return nil
function CatalogService:load()
  local detail = setmetatable({name = "detail", secret = "detail-private"}, Detail)
  local product = setmetatable({id = 7, name = "Anvil", detail = detail, items = {detail}, secret = "product-private"}, Product)
  local copied = self.product_converter:copy(product)
  local summary = self.product_converter:map(copied)
  self.result = string.format("%s %d: %s, %s, %d", summary.kind, summary.id, summary.name, summary.detail.name, #summary.items)
end
