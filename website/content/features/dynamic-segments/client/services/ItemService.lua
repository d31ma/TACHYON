-- @Service
---@class ItemService
ItemService = {}

---@param path string
---@return string
function ItemService:id_from(path)
  return path:match("[^/]*$")
end
