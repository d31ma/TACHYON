-- @Service
---@class CatalogService
CatalogService = {
  -- @Needs
  ---@type CatalogClient
  catalog_client = nil,
  ---@type string[]
  products = {},
  ---@type string
  error = "",
}

---@return nil
function CatalogService:load()
  local loaded, answer = pcall(self.catalog_client.all, self.catalog_client)
  self:_show(loaded, answer)
end

---@param loaded boolean
---@param answer string[]
---@return nil
function CatalogService:_show(loaded, answer)
  if not loaded then
    self.error = "Products could not be loaded. Please retry."
    return
  end
  self.products = answer
  self.error = ""
end
