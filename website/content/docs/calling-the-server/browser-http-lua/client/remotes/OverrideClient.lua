-- @HTTPClient("/api")
---@class OverrideClient
OverrideClient = {}

---@return Greeting
-- @GetRequest("/never")
function OverrideClient:read()
end
