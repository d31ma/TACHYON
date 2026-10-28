-- @Delegate
---@class CatalogEchoDelegate
CatalogEchoDelegate = {}

---@param request table
---@param execution table
---@return table
function CatalogEchoDelegate:exchange(request, execution)
  return { method = request.method, path = request.path, query = request.query, headers = request.headers, body = request.body }
end
