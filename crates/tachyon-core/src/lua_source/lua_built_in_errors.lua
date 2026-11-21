BrokenRule = { __name = "BrokenRule" }
BrokenRule.__index = BrokenRule

NotSignedIn = { __name = "NotSignedIn", status = 401 }
NotSignedIn.__index = NotSignedIn

RoleMissing = { __name = "RoleMissing", status = 403 }
RoleMissing.__index = RoleMissing

IdentityCheckFailed = { __name = "IdentityCheckFailed", status = 500 }
IdentityCheckFailed.__index = IdentityCheckFailed

RequestRejected = { __name = "RequestRejected", status = 400 }
RequestRejected.__index = RequestRejected

BodyTooLarge = { __name = "BodyTooLarge", status = 413 }
BodyTooLarge.__index = BodyTooLarge

CheckRejected = { __name = "CheckRejected", status = 422 }
CheckRejected.__index = CheckRejected

UnexpectedError = { __name = "UnexpectedError", status = 500 }
UnexpectedError.__index = UnexpectedError

YonBuiltInErrors = {
  NO_ROUTE = "No controller method answers this route.",
}

function YonBuiltInErrors.of(value)
  local fields = type(value) == "table" and value or {}
  local known = YonBuiltInErrors.TYPES[fields.type]
  if known == nil then
    error(setmetatable({ message = YonBuiltInErrors.NO_ROUTE }, YonNoRoute), 0)
  end
  return setmetatable(known[2](fields), known[1])
end

function YonBuiltInErrors.text(value)
  return type(value) == "string" and value or ""
end

function YonBuiltInErrors.reference(value)
  return { reference = YonBuiltInErrors.text(value.reference) }
end

function YonBuiltInErrors.request_rejected(value)
  local fields = YonBuiltInErrors.reference(value)
  fields.part = YonBuiltInErrors.text(value.part)
  fields.field = YonBuiltInErrors.text(value.field)
  fields.message = YonBuiltInErrors.text(value.message)
  return fields
end

function YonBuiltInErrors.body_too_large(value)
  local fields = YonBuiltInErrors.reference(value)
  local limit = value.limitBytes
  fields.limit_bytes = type(limit) == "number" and math.tointeger(limit) or 0
  return fields
end

function YonBuiltInErrors.check_rejected(value)
  local fields = YonBuiltInErrors.reference(value)
  fields.violations = YonBuiltInErrors.rules(value.violations)
  fields.truncated = value.truncated == true
  return fields
end

function YonBuiltInErrors.rules(value)
  local rules = {}
  for index, item in ipairs(type(value) == "table" and value or {}) do
    local rule = type(item) == "table" and item or {}
    rules[index] = setmetatable({
      field = YonBuiltInErrors.text(rule.field),
      rule = YonBuiltInErrors.text(rule.rule),
    }, BrokenRule)
  end
  return rules
end

YonBuiltInErrors.TYPES = {
  NotSignedIn = { NotSignedIn, YonBuiltInErrors.reference },
  RoleMissing = { RoleMissing, YonBuiltInErrors.reference },
  IdentityCheckFailed = { IdentityCheckFailed, YonBuiltInErrors.reference },
  RequestRejected = { RequestRejected, YonBuiltInErrors.request_rejected },
  BodyTooLarge = { BodyTooLarge, YonBuiltInErrors.body_too_large },
  CheckRejected = { CheckRejected, YonBuiltInErrors.check_rejected },
  UnexpectedError = { UnexpectedError, YonBuiltInErrors.reference },
}

