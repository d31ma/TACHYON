-- @HTTPClient
---@class CatalogClient
CatalogClient = {}

---@param page integer
---@return table
-- @GetRequest("/products")
function CatalogClient:all(
  -- @RequestParam("page")
  page
)
end

---@param product table
---@return table
-- @PostRequest("/products")
function CatalogClient:add(
  -- @RequestBody
  product
)
end
