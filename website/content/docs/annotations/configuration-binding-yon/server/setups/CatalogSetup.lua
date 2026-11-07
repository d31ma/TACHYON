-- @Setup
-- @Settings({"prefix":"YON_CATALOG_"})
-- @WhenProfile("development")
-- @WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})
---@class CatalogSetup
CatalogSetup = {
  -- @Min(1)
  -- @Max(100)
  ---@type integer
  page_size = 0,
  ---@type boolean
  cache_enabled = false,
}

---@param execution table
---@return table
function CatalogSetup.configure(execution)
  return {}
end
