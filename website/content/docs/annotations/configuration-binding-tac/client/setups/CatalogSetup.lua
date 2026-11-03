-- @Setup
-- @Settings({"prefix":"TAC_CATALOG_"})
-- @WhenProfile("public")
-- @WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
---@class CatalogSetup
CatalogSetup = {
  -- @Min(1)
  -- @Max(100)
  ---@type integer
  page_size = 20,
  ---@type boolean
  cache_enabled = false,
  ---@type string
  title = "Catalog",
}

---@param execution table
---@return table
function CatalogSetup.configure(execution)
  return {}
end
