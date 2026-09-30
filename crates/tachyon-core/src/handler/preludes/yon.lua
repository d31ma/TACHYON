YonLimits = {
  MAX_FRAME_BYTES = 16 * 1024 * 1024,
  MAX_MESSAGE_CHARACTERS = 2048,
  PROTOCOL_ERROR_EXIT_CODE = 70,
  JSON_CONTENT_TYPE = "application/json; charset=utf-8",
  MISSING_METHOD = 405,
  MIDDLEWARE_PHASES = { ["middleware.before"] = "before", ["middleware.after"] = "after" },
  PASSED = { 204, {}, {} },
}

YonBase64 = {
  LETTERS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
}

function YonBase64.decode(text)
  local bits = text:gsub("[^%w%+/]", ""):gsub(".", function(letter)
    local value = YonBase64.LETTERS:find(letter, 1, true) - 1
    local binary = {}
    for shift = 5, 0, -1 do
      binary[#binary + 1] = (value >> shift) & 1
    end
    return table.concat(binary)
  end)
  return (bits:gsub("%d%d%d?%d?%d?%d?%d?%d?", function(byte)
    if #byte ~= 8 then
      return ""
    end
    return string.char(tonumber(byte, 2))
  end))
end

YonRequest = {}
YonRequest.__index = YonRequest

function YonRequest.of(frame)
  local headers = {}
  for name, values in pairs(frame.headers or {}) do
    headers[string.lower(name)] = values[1]
  end
  return setmetatable({
    method = frame.method,
    path = frame.target or frame.route or "/",
    headers = headers,
    body = YonRequest.body(frame.body),
  }, YonRequest)
end

function YonRequest.body(body)
  if body == nil then
    return ""
  end
  if body.encoding == "base64" then
    return YonBase64.decode(body.data or "")
  end
  return body.data or ""
end

function YonRequest:header(name)
  return self.headers[string.lower(name)]
end

function YonRequest:query(name)
  local query = self.path:match("%?(.*)$") or ""
  for pair in query:gmatch("[^&]+") do
    local key, value = pair:match("^([^=]*)=?(.*)$")
    if key == name then
      return value
    end
  end
  return nil
end

YonFailures = {}

function YonFailures.message(failure)
  if type(failure) == "table" then
    return tostring(failure.message or failure)
  end
  return tostring(failure)
end

function YonFailures.stop(failure)
  io.stderr:write(YonFailures.message(failure), "\n")
  io.stderr:flush()
  os.exit(1)
end

YonAnswers = {}

function YonAnswers.is_status(value)
  return math.type(value) == "integer" and value >= 100 and value <= 599
end

function YonAnswers.is_answer(value)
  return type(value) == "table" and getmetatable(value) == nil and #value == 3
    and YonAnswers.is_status(value[1]) and type(value[2]) == "table"
end

function YonAnswers.for_result(result)
  if YonAnswers.is_answer(result) then
    return YonAnswers.explicit(result)
  end
  return YonAnswers.envelope(200, { ["content-type"] = { YonLimits.JSON_CONTENT_TYPE } }, YonJson.encode(result))
end

function YonAnswers.explicit(answer)
  return YonAnswers.envelope(answer[1], YonAnswers.headers(answer[2]), YonAnswers.text(answer[3]))
end

function YonAnswers.empty(status)
  return YonAnswers.envelope(status, {}, nil)
end

function YonAnswers.envelope(status, headers, body)
  return { status = status, headers = setmetatable(headers, YonJson.OBJECT), body = body }
end

function YonAnswers.headers(raw)
  local headers = {}
  for name, value in pairs(raw) do
    if type(name) ~= "string" then
      YonJson.shape_error("Handler response header names must be strings.")
    end
    local values = type(value) == "table" and value or { value }
    for _, item in ipairs(values) do
      if type(item) ~= "string" then
        YonJson.shape_error("Handler response header '" .. name .. "' must contain strings.")
      end
    end
    headers[string.lower(name)] = values
  end
  return headers
end

function YonAnswers.text(body)
  if body == nil then
    return ""
  end
  if type(body) == "string" then
    return body
  end
  if type(body) == "table" and YonJson.is_list(body) then
    local joined = {}
    for index, part in ipairs(body) do
      if type(part) ~= "string" then
        return YonJson.encode(body)
      end
      joined[index] = part
    end
    return table.concat(joined)
  end
  return YonJson.encode(body)
end

YonErrors = {}

function YonErrors.handle(failure, candidates)
  local selected = YonErrors.nearest(failure, candidates)
  if selected == nil then
    error(failure, 0)
  end
  return YonErrors.answer(selected[2], selected[3]())
end

function YonErrors.distance(failure, error_type)
  local current = type(failure) == "table" and getmetatable(failure) or nil
  local distance = 0
  while current ~= nil do
    if current == error_type then
      return distance
    end
    local parent = getmetatable(current)
    current = parent and parent.__index or nil
    distance = distance + 1
  end
  return nil
end

function YonErrors.nearest(failure, candidates)
  local selected = nil
  local best = nil
  for _, candidate in ipairs(candidates) do
    local distance = YonErrors.distance(failure, candidate[1])
    if distance ~= nil and (best == nil or distance < best) then
      selected = candidate
      best = distance
    end
  end
  return selected
end

function YonErrors.answer(status, result)
  if YonAnswers.is_answer(result) then
    local answered = result[1] == 200 and status or result[1]
    return { answered, result[2], result[3] }
  end
  return { status, { ["content-type"] = "application/json" }, { YonJson.encode(result) } }
end

YonInput = {}

function YonInput.read()
  local input = io.stdin:read("a") or ""
  if #input > YonLimits.MAX_FRAME_BYTES then
    YonFailures.stop("The request exceeds the protocol frame limit.")
  end
  local ok, envelope = pcall(YonJson.decode, input)
  if not ok or type(envelope) ~= "table" then
    os.exit(YonLimits.PROTOCOL_ERROR_EXIT_CODE)
  end
  return envelope
end

YonInvocation = {}

function YonInvocation.answer(handler, request)
  local phase = YonLimits.MIDDLEWARE_PHASES[request.operation]
  if phase ~= nil then
    return YonInvocation.middleware(handler, request, phase)
  end
  local method = rawget(handler, tostring(request.method))
  if type(method) ~= "function" then
    return YonAnswers.empty(YonLimits.MISSING_METHOD)
  end
  local argument = request
  if not rawget(handler, "YON_FRAMES") then
    argument = YonRequest.of(request)
  end
  return YonAnswers.for_result(method(argument))
end

function YonInvocation.middleware(handler, request, phase)
  local method = rawget(handler, phase)
  if method == nil then
    return YonAnswers.for_result(YonLimits.PASSED)
  end
  local result = method(YonRequest.of(request))
  if result == nil then
    return YonAnswers.for_result(YonLimits.PASSED)
  end
  if not YonAnswers.is_answer(result) then
    YonFailures.stop("Middleware " .. phase .. "() must return a { status, headers, body } list, or nil to let the request pass.")
  end
  return YonAnswers.for_result(result)
end

YonRunner = {}

function YonRunner.main(program)
  local standard_output = io.stdout
  io.output(io.stderr)
  print = YonRunner.print
  local request = YonInput.read()
  local answer = YonRunner.run(program, request)
  local ok, text = pcall(YonJson.encode, answer)
  if not ok then
    YonFailures.stop(YonJson.NOT_SERIALIZABLE)
  end
  standard_output:write(text)
  standard_output:flush()
  os.exit(0)
end

function YonRunner.print(...)
  local parts = table.pack(...)
  for index = 1, parts.n do
    parts[index] = tostring(parts[index])
  end
  io.stderr:write(table.concat(parts, "\t", 1, parts.n), "\n")
end

function YonRunner.run(program, request)
  local loaded, failure = pcall(dofile, program)
  if not loaded then
    YonFailures.stop(failure)
  end
  local name = "Handler"
  if YonLimits.MIDDLEWARE_PHASES[request.operation] ~= nil then
    name = "Middleware"
  end
  local handler = rawget(_G, name)
  if type(handler) ~= "table" then
    YonFailures.stop("Module must define a table named " .. name .. ".")
  end
  local ok, answer = xpcall(YonInvocation.answer, debug.traceback, handler, request)
  if not ok then
    YonFailures.stop(answer)
  end
  return answer
end

YonRunner.main(arg[1])
