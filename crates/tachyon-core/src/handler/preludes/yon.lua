YonLimits = {
  MAX_FRAME_BYTES = 16 * 1024 * 1024,
  MAX_MESSAGE_CHARACTERS = 2048,
  PROTOCOL_ERROR_EXIT_CODE = 70,
  MAX_DEPTH = 64,
  JSON_CONTENT_TYPE = "application/json; charset=utf-8",
  NOT_SERIALIZABLE = "Handler returned a value that is not JSON-serializable.",
  MISSING_METHOD = 405,
  MIDDLEWARE_PHASES = { ["middleware.before"] = "before", ["middleware.after"] = "after" },
  PASSED = { 204, {}, {} },
}

YonShapeError = {}
YonShapeError.__index = YonShapeError

YonNoRoute = {}
YonNoRoute.__index = YonNoRoute

YonNull = setmetatable({}, { __name = "null" })

YonJson = {
  OBJECT = { __name = "object" },
  ESCAPES = { ['"'] = '\\"', ["\\"] = "\\\\", ["\b"] = "\\b", ["\f"] = "\\f", ["\n"] = "\\n", ["\r"] = "\\r", ["\t"] = "\\t" },
  UNESCAPES = { ['"'] = '"', ["\\"] = "\\", ["/"] = "/", b = "\b", f = "\f", n = "\n", r = "\r", t = "\t" },
}

function YonJson.shape_error(message)
  error(setmetatable({ message = message }, YonShapeError), 0)
end

function YonJson.encode(value)
  local parts = {}
  YonJson.write(parts, value, 0)
  return table.concat(parts)
end

function YonJson.write(parts, value, depth)
  if depth > YonLimits.MAX_DEPTH then
    YonJson.shape_error(YonLimits.NOT_SERIALIZABLE)
  end
  local kind = type(value)
  if value == nil or value == YonNull then
    parts[#parts + 1] = "null"
  elseif kind == "boolean" then
    parts[#parts + 1] = tostring(value)
  elseif kind == "number" then
    parts[#parts + 1] = YonJson.number(value)
  elseif kind == "string" then
    parts[#parts + 1] = YonJson.text(value)
  elseif kind == "table" then
    YonJson.write_table(parts, value, depth)
  else
    YonJson.shape_error(YonLimits.NOT_SERIALIZABLE)
  end
end

function YonJson.number(value)
  if math.type(value) == "integer" then
    return string.format("%d", value)
  end
  if value ~= value or value == math.huge or value == -math.huge then
    YonJson.shape_error(YonLimits.NOT_SERIALIZABLE)
  end
  if value == math.floor(value) and math.abs(value) < 1e15 then
    return string.format("%.1f", value)
  end
  return string.format("%.17g", value)
end

function YonJson.text(value)
  if utf8.len(value) == nil then
    YonJson.shape_error(YonLimits.NOT_SERIALIZABLE)
  end
  local escaped = value:gsub('[%c"\\]', function(character)
    return YonJson.ESCAPES[character] or string.format("\\u%04x", character:byte())
  end)
  return '"' .. escaped .. '"'
end

function YonJson.write_table(parts, value, depth)
  local fields = YonModels.fields_of(value)
  if fields ~= nil then
    YonJson.write_fields(parts, value, fields, depth)
  elseif getmetatable(value) ~= YonJson.OBJECT and YonJson.is_list(value) then
    YonJson.write_list(parts, value, depth)
  else
    YonJson.write_fields(parts, value, YonJson.sorted_keys(value), depth)
  end
end

function YonJson.is_list(value)
  local count = 0
  for key in pairs(value) do
    if math.type(key) ~= "integer" or key < 1 then
      return false
    end
    count = count + 1
  end
  return count == #value
end

function YonJson.sorted_keys(value)
  local keys = {}
  for key in pairs(value) do
    if type(key) ~= "string" then
      YonJson.shape_error(YonLimits.NOT_SERIALIZABLE)
    end
    keys[#keys + 1] = key
  end
  table.sort(keys)
  return keys
end

function YonJson.write_list(parts, value, depth)
  parts[#parts + 1] = "["
  for index = 1, #value do
    if index > 1 then
      parts[#parts + 1] = ","
    end
    YonJson.write(parts, value[index], depth + 1)
  end
  parts[#parts + 1] = "]"
end

function YonJson.write_fields(parts, value, keys, depth)
  parts[#parts + 1] = "{"
  for index, key in ipairs(keys) do
    if index > 1 then
      parts[#parts + 1] = ","
    end
    parts[#parts + 1] = YonJson.text(key)
    parts[#parts + 1] = ":"
    YonJson.write(parts, value[key], depth + 1)
  end
  parts[#parts + 1] = "}"
end

function YonJson.decode(text)
  local reader = { text = text, at = 1 }
  local value = YonJson.read(reader, 0)
  YonJson.skip(reader)
  if reader.at <= #text then
    YonJson.fail(reader)
  end
  return value
end

function YonJson.fail(reader)
  error("Invalid JSON at byte " .. reader.at, 0)
end

function YonJson.skip(reader)
  reader.at = reader.text:find("[^ \t\r\n]", reader.at) or (#reader.text + 1)
end

function YonJson.read(reader, depth)
  if depth > YonLimits.MAX_DEPTH then
    YonJson.fail(reader)
  end
  YonJson.skip(reader)
  local character = reader.text:sub(reader.at, reader.at)
  if character == "{" then
    return YonJson.read_object(reader, depth)
  elseif character == "[" then
    return YonJson.read_array(reader, depth)
  elseif character == '"' then
    return YonJson.read_text(reader)
  end
  return YonJson.read_word(reader)
end

function YonJson.read_word(reader)
  local word = reader.text:match("^[%w%.%+%-]+", reader.at)
  if word == nil then
    YonJson.fail(reader)
  end
  reader.at = reader.at + #word
  if word == "true" then
    return true
  elseif word == "false" then
    return false
  elseif word == "null" then
    return nil
  end
  local number = tonumber(word)
  if number == nil or not word:match("^%-?%d") then
    YonJson.fail(reader)
  end
  return number
end

function YonJson.read_text(reader)
  local parts = {}
  local at = reader.at + 1
  while true do
    local stop = reader.text:find('["\\]', at)
    if stop == nil then
      YonJson.fail(reader)
    end
    parts[#parts + 1] = reader.text:sub(at, stop - 1)
    if reader.text:sub(stop, stop) == '"' then
      reader.at = stop + 1
      return table.concat(parts)
    end
    at = YonJson.read_escape(reader, parts, stop + 1)
  end
end

function YonJson.read_escape(reader, parts, at)
  local letter = reader.text:sub(at, at)
  if letter ~= "u" then
    parts[#parts + 1] = YonJson.UNESCAPES[letter] or YonJson.fail(reader)
    return at + 1
  end
  local code = tonumber(reader.text:sub(at + 1, at + 4), 16) or YonJson.fail(reader)
  local after = at + 5
  if code >= 0xD800 and code <= 0xDBFF and reader.text:sub(after, after + 1) == "\\u" then
    local low = tonumber(reader.text:sub(after + 2, after + 5), 16) or YonJson.fail(reader)
    code = 0x10000 + (code - 0xD800) * 0x400 + (low - 0xDC00)
    after = after + 6
  end
  parts[#parts + 1] = utf8.char(code)
  return after
end

function YonJson.read_array(reader, depth)
  local list = {}
  reader.at = reader.at + 1
  YonJson.skip(reader)
  if reader.text:sub(reader.at, reader.at) == "]" then
    reader.at = reader.at + 1
    return list
  end
  while true do
    list[#list + 1] = YonJson.read(reader, depth + 1)
    YonJson.skip(reader)
    local character = reader.text:sub(reader.at, reader.at)
    reader.at = reader.at + 1
    if character == "]" then
      return list
    elseif character ~= "," then
      YonJson.fail(reader)
    end
  end
end

function YonJson.read_object(reader, depth)
  local object = {}
  reader.at = reader.at + 1
  YonJson.skip(reader)
  if reader.text:sub(reader.at, reader.at) == "}" then
    reader.at = reader.at + 1
    return object
  end
  while true do
    YonJson.skip(reader)
    if reader.text:sub(reader.at, reader.at) ~= '"' then
      YonJson.fail(reader)
    end
    local key = YonJson.read_text(reader)
    YonJson.skip(reader)
    if reader.text:sub(reader.at, reader.at) ~= ":" then
      YonJson.fail(reader)
    end
    reader.at = reader.at + 1
    object[key] = YonJson.read(reader, depth + 1)
    YonJson.skip(reader)
    local character = reader.text:sub(reader.at, reader.at)
    reader.at = reader.at + 1
    if character == "}" then
      return object
    elseif character ~= "," then
      YonJson.fail(reader)
    end
  end
end

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

YonModels = {
  field_order = {},
  field_types = {},
}

function YonModels.fields_of(value)
  local model = getmetatable(value)
  if model == nil then
    return nil
  end
  return YonModels.field_order[model]
end

function YonModels.read(descriptor, value)
  if type(value) ~= "table" or descriptor == nil then
    return value
  end
  if descriptor.list then
    for index = 1, #value do
      value[index] = YonModels.read(descriptor.item, value[index])
    end
    return value
  end
  local model = descriptor.model
  for name, field in pairs(YonModels.field_types[model] or {}) do
    value[name] = YonModels.read(field, value[name])
  end
  return setmetatable(value, model)
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
    YonFailures.stop(YonLimits.NOT_SERIALIZABLE)
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
