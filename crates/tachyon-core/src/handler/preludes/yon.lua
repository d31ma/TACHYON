YonLimits = {
  MAX_FRAME_BYTES = 16 * 1024 * 1024,
  MAX_MESSAGE_CHARACTERS = 2048,
  PROTOCOL_ERROR_EXIT_CODE = 70,
  JSON_CONTENT_TYPE = "application/json; charset=utf-8",
  MISSING_METHOD = 405,
  MIDDLEWARE_PHASES = { ["middleware.before"] = "before", ["middleware.after"] = "after" },
  DELIVERY = "event.deliver",
  MISSING_LISTENERS = 404,
  PASSED = { 204, {}, {} },
  UNREADABLE_MESSAGE = "The socket message is not valid JSON, or it is nested more than %d levels deep: %s",
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
  local input = YonInput.text()
  if #input > YonLimits.MAX_FRAME_BYTES then
    YonFailures.stop("The request exceeds the protocol frame limit.")
  end
  local ok, envelope = pcall(YonJson.decode, input)
  if not ok or type(envelope) ~= "table" then
    os.exit(YonLimits.PROTOCOL_ERROR_EXIT_CODE)
  end
  return envelope
end

function YonInput.text()
  if os.getenv("YON_SOCKET_FRAMES") == "1" then
    return YonFrames.read() or ""
  end
  return io.stdin:read("a") or ""
end

YonFrames = { output = io.stdout }

function YonFrames.read()
  local prefix = io.stdin:read(4)
  if prefix == nil or #prefix < 4 then
    return nil
  end
  local length = string.unpack(">I4", prefix)
  if length > YonLimits.MAX_FRAME_BYTES then
    os.exit(YonLimits.PROTOCOL_ERROR_EXIT_CODE)
  end
  if length == 0 then
    return ""
  end
  return io.stdin:read(length)
end

function YonFrames.write(payload)
  YonFrames.output:write(string.pack(">I4", #payload), payload)
  YonFrames.output:flush()
end

SocketPeer = {}
SocketPeer.__index = SocketPeer

function SocketPeer:send(message)
  self.session:send_to(message, self.id)
end

SocketSession = {}
SocketSession.__index = SocketSession

function SocketSession.new(id)
  return setmetatable({ id = id, inbox = {}, closed = false, sessions = nil }, SocketSession)
end

function SocketSession:send(message)
  self:send_to(message, nil)
end

function SocketSession:close()
  self.closed = true
  self:write({ kind = "close" })
end

function SocketSession:open_sessions()
  self.sessions = nil
  self:write({ kind = "sessions" })
  while self.sessions == nil and not self.closed do
    self:receive()
  end
  local peers = {}
  for index, id in ipairs(self.sessions or {}) do
    peers[index] = setmetatable({ session = self, id = tostring(id) }, SocketPeer)
  end
  return peers
end

function SocketSession:send_to(message, to)
  self:write({ kind = "event", to = to, body = { encoding = "utf8", data = YonJson.encode(message) } })
end

function SocketSession:next_message()
  while #self.inbox == 0 and not self.closed do
    self:receive()
  end
  return table.remove(self.inbox, 1)
end

function SocketSession:receive()
  local raw = YonFrames.read()
  local ok, frame = pcall(YonJson.decode, raw or "")
  if raw == nil or not ok or type(frame) ~= "table" then
    self.closed = true
    return
  end
  if frame.request_id ~= self.id then
    os.exit(YonLimits.PROTOCOL_ERROR_EXIT_CODE)
  end
  self:take(frame)
end

function SocketSession:take(frame)
  if frame.kind == "message" then
    self.inbox[#self.inbox + 1] = SocketSession.read_message((frame.body or {}).data or "null")
  elseif frame.kind == "sessions" then
    self.sessions = type(frame.sessions) == "table" and frame.sessions or {}
  else
    self.closed = true
  end
end

function SocketSession.read_message(data)
  local ok, value = pcall(YonJson.decode, data)
  if not ok then
    return { failure = string.format(YonLimits.UNREADABLE_MESSAGE, YonJson.MAX_DEPTH, YonFailures.message(value)) }
  end
  return { value = value }
end

function SocketSession:write(frame)
  frame.protocol_version = 1
  frame.request_id = self.id
  YonFrames.write(YonJson.encode(frame))
end

YonSocket = {}

function YonSocket.run(endpoint, request)
  local session = SocketSession.new(request.request_id)
  YonSocket.attempt(endpoint, session, endpoint.open, session)
  while true do
    local message = session:next_message()
    if message == nil then
      break
    elseif message.failure ~= nil then
      YonSocket.report(endpoint, session, message.failure)
    else
      YonSocket.attempt(endpoint, session, endpoint.message, session, message.value)
    end
  end
  pcall(endpoint.close, endpoint, session)
  os.exit(0)
end

function YonSocket.attempt(endpoint, session, action, ...)
  local ok, failure = pcall(action, endpoint, ...)
  if not ok then
    YonSocket.report(endpoint, session, failure)
  end
end

function YonSocket.report(endpoint, session, failure)
  io.stderr:write(YonJson.encode({ event = "handler.socket_failed", request_id = session.id, message = YonFailures.message(failure) }), "\n")
  pcall(endpoint.error, endpoint, session, failure)
end

YonTopics = { sent = {} }

function YonTopics.send(topic, payload_json)
  YonTopics.sent[#YonTopics.sent + 1] = { topic = topic, payload_json = payload_json }
end

function YonTopics.add_to(answer)
  if #YonTopics.sent > 0 and getmetatable(answer) ~= YonStream then
    answer.topics = YonTopics.sent
  end
  return answer
end

YonInvocation = {}

function YonInvocation.answer(handler, request)
  local phase = YonLimits.MIDDLEWARE_PHASES[request.operation]
  if phase ~= nil then
    return YonInvocation.middleware(handler, request, phase)
  end
  if request.operation == YonLimits.DELIVERY then
    return YonInvocation.deliver(handler, request)
  end
  local method = rawget(handler, tostring(request.method))
  if type(method) ~= "function" then
    return YonAnswers.empty(YonLimits.MISSING_METHOD)
  end
  local argument = request
  if not rawget(handler, "YON_FRAMES") then
    argument = YonRequest.of(request)
  end
  local result = method(argument)
  if getmetatable(result) == YonStream then
    return result
  end
  return YonAnswers.for_result(result)
end

function YonInvocation.deliver(handler, request)
  local method = rawget(handler, "yon_deliver")
  if type(method) ~= "function" then
    return YonAnswers.empty(YonLimits.MISSING_LISTENERS)
  end
  method(YonJson.decode(YonRequest.body(request.body)))
  return YonAnswers.for_result(nil)
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

YonStream = {
  MAX_FRAME_BYTES = 16 * 1024 * 1024,
  FAILED = "The handler stream failed.",
  TOO_LARGE = "Streamed event exceeds the protocol frame limit.",
}
YonStream.__index = YonStream

function YonStream.of(body)
  return setmetatable({ body = body }, YonStream)
end

function YonStream:send(output, request_id)
  local routine = coroutine.create(self.body)
  while true do
    local ok, event = coroutine.resume(routine)
    if not ok then
      io.stderr:write(YonFailures.message(event), "\n")
      YonStream.failure(output, request_id, "TY2204", YonStream.FAILED)
    end
    if coroutine.status(routine) == "dead" then
      output:flush()
      os.exit(0)
    end
    YonStream.event(output, request_id, event)
  end
end

function YonStream.event(output, request_id, event)
  local encoded, data = pcall(YonJson.encode, event)
  if not encoded then
    YonStream.failure(output, request_id, "TY2203", YonJson.NOT_SERIALIZABLE)
  end
  local payload = YonJson.encode({
    protocol_version = 1,
    kind = "event",
    request_id = request_id,
    body = { encoding = "utf8", data = data },
  })
  if #payload > YonStream.MAX_FRAME_BYTES then
    YonStream.failure(output, request_id, "TY2203", YonStream.TOO_LARGE)
  end
  YonStream.frame(output, payload)
end

function YonStream.failure(output, request_id, code, message)
  YonStream.frame(output, YonJson.encode({
    protocol_version = 1,
    kind = "response",
    request_id = request_id,
    status = 500,
    headers = setmetatable({}, YonJson.OBJECT),
    error = { code = code, message = message, retryable = false },
  }))
  output:flush()
  os.exit(0)
end

function YonStream.frame(output, payload)
  output:write(string.pack(">I4", #payload), payload)
  output:flush()
end

YonRunner = {}

function YonRunner.main(program)
  local standard_output = io.stdout
  YonFrames.output = standard_output
  io.output(io.stderr)
  print = YonRunner.print
  local request = YonInput.read()
  local answer = YonTopics.add_to(YonRunner.run(program, request))
  if getmetatable(answer) == YonStream then
    answer:send(standard_output, request.request_id)
  end
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
