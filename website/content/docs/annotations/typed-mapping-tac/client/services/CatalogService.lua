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
  self.result = YonJson.encode(self.product_converter:map(copied))
end
