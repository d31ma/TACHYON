-- @HTTPClient("/api")
-- @CacheSettings({"name":"greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
---@class GreetingClient
GreetingClient = {}

---@return Greeting
-- @Cached
-- @GetRequest("/greeting")
function GreetingClient:read()
end

---@return Greeting
-- @ClearCache({"method":"read"})
-- @PostRequest("/clear")
function GreetingClient:clear()
end
