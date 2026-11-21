-- @ErrorHandler
---@class OrdersErrorHandler
OrdersErrorHandler = {}

---@param failure NotSignedIn
---@return Problem
-- @HandlesError
function OrdersErrorHandler:sign_in_needed(failure)
  return { code = "not_signed_in", message = "[MS0001]: Sign in again." }
end

---@param failure RoleMissing
---@return Problem
-- @HandlesError
function OrdersErrorHandler:not_allowed(failure)
  return { code = "forbidden", message = "[MS0002]: You may not do this." }
end

---@param failure IdentityCheckFailed
---@return Problem
-- @HandlesError
function OrdersErrorHandler:identity_down(failure)
  return { code = "identity_failed", message = "[MS0003]: Try again later." }
end

---@param failure RequestRejected
---@return Problem
-- @HandlesError
function OrdersErrorHandler:bad_request(failure)
  return { code = "invalid_request", message = "[MS0004]: " .. failure.part .. "." .. failure.field }
end

---@param failure BodyTooLarge
---@return Problem
-- @HandlesError
function OrdersErrorHandler:too_large(failure)
  return { code = "too_large", message = "[MS0005]: " .. failure.limit_bytes }
end

---@param failure CheckRejected
---@return Problem
-- @HandlesError
function OrdersErrorHandler:broken_rules(failure)
  return { code = "invalid_input", message = "[MS0006]: " .. self:_rules(failure.violations) }
end

---@param failure UnexpectedError
---@return Problem
-- @HandlesError
function OrdersErrorHandler:unexpected(failure)
  return { code = "unexpected", message = "[MS0000]: Something went wrong." }
end

---@param violations table
---@return string
function OrdersErrorHandler:_rules(violations)
  local names = {}
  for index, violation in ipairs(violations) do
    names[index] = violation.field .. ":" .. violation.rule
  end
  return table.concat(names, ",")
end
