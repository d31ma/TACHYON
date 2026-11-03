-- @IdentityProvider
---@class AppIdentityProvider
AppIdentityProvider = {}

---@param request table
---@return Identity|nil
function AppIdentityProvider:identify(request)
  return self:_find(request.headers["authorization"])
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
