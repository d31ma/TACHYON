-- @HTTPClient("/api")
---@class CatalogClient
CatalogClient = {}

---@param id string
---@param view string
---@param tenant string
---@return table
-- @GetRequest("/products/{id}")
function CatalogClient:find(
  -- @PathVariable("id")
  id,
  -- @RequestParam("view")
  view,
  -- @RequestHeader("X-Tenant")
  tenant
)
end

---@param product table
---@return table
-- @PostRequest("/products")
function CatalogClient:save(
  -- @RequestBody
  product
)
end

---@param id integer
---@param product table
---@return table
-- @PutRequest("/products/{id}")
function CatalogClient:replace(
  -- @PathVariable("id")
  id,
  -- @RequestBody
  product
)
end

---@param id integer
---@param product table
---@return table
-- @PatchRequest("/products/{id}")
function CatalogClient:rename(
  -- @PathVariable("id")
  id,
  -- @RequestBody
  product
)
end

---@param id integer
---@return table
-- @DeleteRequest("/products/{id}")
function CatalogClient:remove(
  -- @PathVariable("id")
  id
)
end
