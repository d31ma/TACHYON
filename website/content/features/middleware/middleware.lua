Middleware = {}

function Middleware.before(request)
  local is_private = request.path:sub(1, #"/private") == "/private"
  if is_private and request.headers["authorization"] ~= "let-me-in" then
    return { 401, { ["content-type"] = "application/json" }, '{"error":"Unauthorized"}' }
  end
  return nil
end

function Middleware.after(request)
  return { 204, { ["x-served-by"] = "tachyon" }, "" }
end
