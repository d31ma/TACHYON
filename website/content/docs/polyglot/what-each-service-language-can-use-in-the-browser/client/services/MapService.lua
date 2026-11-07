-- @Service
---@class MapService
MapService = {
  ---@type number
  latitude = 0.0,
  ---@type number
  longitude = 0.0,
}

---@param value PlaceFound
---@return nil
-- @OnEvent
function MapService:show(value)
  self.latitude = value.latitude
  self.longitude = value.longitude
end
