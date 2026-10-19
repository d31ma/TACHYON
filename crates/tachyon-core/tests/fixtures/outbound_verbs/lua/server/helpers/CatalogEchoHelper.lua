-- @Helper
---@class CatalogEchoHelper
CatalogEchoHelper = {}

---@param request table
---@param execution table
---@return table
function CatalogEchoHelper:exchange(request, execution)
  return { method = request.method, path = request.path, query = request.query, headers = request.headers, body = request.body }
end
