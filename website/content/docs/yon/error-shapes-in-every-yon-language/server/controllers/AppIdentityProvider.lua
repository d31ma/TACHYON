-- @IdentityProvider
---@class AppIdentityProvider
AppIdentityProvider = {}

---@param request table
---@return Identity|nil
function AppIdentityProvider:identify(request)
  local token = request.headers["authorization"]
  self:_refuse_broken(token)
  return self:_find(token)
end

---@param token string|nil
function AppIdentityProvider:_refuse_broken(token)
  if token == "broken-token" then
    error("the session store is down")
  end
end

---@param token string|nil
---@return Identity|nil
function AppIdentityProvider:_find(token)
  if token == "ada-token" then
    return { user = "ada", roles = { "Admin" } }
  end
  if token == "bob-token" then
    return { user = "bob", roles = {} }
  end
  return nil
end
