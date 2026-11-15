-- @RequestRoute("/me")
-- @SignedIn
---@class MeController
MeController = {}

---@param caller Identity
---@return string
-- @GetRoute
function MeController:get_me(
  -- @FromIdentity
  caller
)
  return caller.user
end
