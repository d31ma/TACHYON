TacLimits = {
  MAX_OWNERS = 256,
  MAX_QUEUED_EVENTS = 128,
  MAX_REQUEST_BYTES = 1048576,
  MAX_MEMBER_REQUEST_BYTES = 65536,
  MAX_EVENT_BYTES = 65536,
  MAX_RETURNED_EVENTS = 32,
  OWNER_PATTERN = "^%x%x%x%x%x%x%x%x%-%x%x%x%x%-%x%x%x%x%-%x%x%x%x%-%x%x%x%x%x%x%x%x%x%x%x%x$",
}

warn("@on")

load = (function(original)
  return function(chunk, name, _, environment)
    return original(chunk, name, "t", environment)
  end
end)(load)

TacAnswer = {}

function TacAnswer.value(value)
  if value == nil then
    return { value = YonNull }
  end
  return { value = value }
end

function TacAnswer.failure(message)
  return { error = tostring(message) }
end

function TacAnswer.coded(message, code)
  return { error = message, code = code }
end

function TacAnswer.failure_of(failure)
  local answer = TacAnswer.failure(TacAnswer.message(failure))
  if type(failure) == "table" then
    answer.code = failure.behavior_code
  end
  return answer
end

function TacAnswer.message(failure)
  if type(failure) == "table" then
    return tostring(failure.message or failure)
  end
  return tostring(failure)
end

function TacAnswer.too_large(subject, size, limit)
  return TacAnswer.failure("The " .. subject .. " is " .. size .. " bytes. The limit is " .. limit .. " bytes. Pass less data to the service.")
end

TacValues = {}

function TacValues.copy(value)
  if type(value) ~= "table" or value == YonNull then
    return value
  end
  local copied = {}
  for key, item in pairs(value) do
    copied[key] = TacValues.copy(item)
  end
  return setmetatable(copied, getmetatable(value))
end

function TacValues.is_field(key, value)
  return type(key) == "string" and type(value) ~= "function" and key:sub(1, 2) ~= "__" and key:find("%l") ~= nil
end

function TacValues.model_name(value)
  local model = type(value) == "table" and getmetatable(value) or nil
  if type(model) ~= "table" then
    return nil
  end
  return rawget(model, "__name")
end

function TacValues.model(name, value)
  local model = type(name) == "string" and rawget(_G, name) or nil
  if type(value) ~= "table" or type(model) ~= "table" then
    return value
  end
  return YonModels.read({ model = model }, value)
end

ApplicationEventPublisher = {}

function ApplicationEventPublisher:publish_event(value, retain)
  local name = TacValues.model_name(value)
  if name == nil then
    return TacServices.event_failure()
  end
  TacServices.publish(name, value, retain)
  return value
end

TacOwner = {}
TacOwner.__index = TacOwner

function TacOwner.new(request, instance, metadata)
  return setmetatable({
    route = request.route,
    service = request.service,
    id = request.owner,
    instance = instance,
    metadata = metadata,
    queue = {},
    disposed = false,
  }, TacOwner)
end

function TacOwner:dispose()
  if self.disposed then
    return
  end
  self.disposed = true
  self.queue = {}
  for _, name in ipairs(self.metadata.stops or {}) do
    self.instance[name](self.instance)
  end
end

function TacOwner:matches(request)
  return self.route == request.route and self.service == request.service
end

TacServices = {
  resources = { application_event_publisher = ApplicationEventPublisher },
  services = {},
  owners = {},
  owner_count = 0,
  executing = nil,
  handling = false,
}

function TacServices.register(name, class, metadata)
  TacServices.services[name] = { class = class, metadata = YonJson.decode(metadata) }
end

function TacServices.contains(list, wanted)
  for _, item in ipairs(list or {}) do
    if item == wanted then
      return true
    end
  end
  return false
end

function TacServices.lua_name(metadata, name)
  return (metadata.names or {})[name] or name
end

function TacServices.construct(class)
  local instance = {}
  for key, value in pairs(class) do
    if TacValues.is_field(key, value) then
      instance[key] = TacValues.copy(value)
    end
  end
  for field, name in pairs(class.__tachyonContracts or {}) do
    local provider = rawget(_G, name)
    if type(provider) ~= "table" then
      error("Missing converter provider " .. name, 0)
    end
    instance[field] = TacServices.construct(provider)
    if type(provider.start_context) == "function" then
      instance[field]:start_context()
    end
  end
  return setmetatable(instance, {
    __index = function(_, key)
      local value = class[key]
      if value == nil then
        return TacServices.resources[key]
      end
      return value
    end,
  })
end

function TacServices.invoke(text)
  if TacServices.handling then
    return YonJson.encode(TacAnswer.failure("Native owner is busy."))
  end
  TacServices.handling = true
  local ok, answer = pcall(TacTasks.handle, text)
  if ok and TacServices.executing ~= nil then
    TacServices.drain(TacServices.executing)
  end
  TacServices.executing = nil
  TacServices.handling = false
  if not ok then
    answer = TacAnswer.failure(TacAnswer.message(answer))
  end
  return TacTasks.encoded(answer)
end

function TacServices.answered(text)
  local ok, answer = pcall(TacServices.handle, text)
  if not ok then
    return TacAnswer.failure_of(answer)
  end
  return answer
end

function TacServices.handle(text)
  if #text > TacLimits.MAX_REQUEST_BYTES then
    return TacAnswer.too_large("request", #text, TacLimits.MAX_REQUEST_BYTES)
  end
  local read, request = pcall(YonJson.decode, text)
  if not read or type(request) ~= "table" then
    return TacAnswer.failure("Invalid companion request.")
  end
  local operation = request.op
  if operation ~= "init" and #text > TacLimits.MAX_MEMBER_REQUEST_BYTES then
    return TacAnswer.too_large("request to " .. tostring(request.name or operation), #text, TacLimits.MAX_MEMBER_REQUEST_BYTES)
  end
  if type(operation) ~= "string" or operation == "" then
    return TacAnswer.failure("Companion request has no operation.")
  end
  if operation == "host-dispose" then
    TacServices.dispose_all()
    return TacAnswer.value(nil)
  end
  return TacServices.identity_problem(request) or TacServices.handle_owned(request, operation)
end

function TacServices.present(value)
  return type(value) == "string" and value ~= ""
end

function TacServices.identity_problem(request)
  if not TacServices.present(request.route) then
    return TacAnswer.coded("Missing companion route.", "TY_NATIVE_ROUTE")
  end
  if not TacServices.present(request.service) then
    return TacAnswer.coded("Missing companion name.", "TY_NATIVE_SERVICE")
  end
  if type(request.owner) ~= "string" or not request.owner:match(TacLimits.OWNER_PATTERN) then
    return TacAnswer.coded("Missing native owner identity.", "TY_NATIVE_OWNER")
  end
  return nil
end

function TacServices.handle_owned(request, operation)
  if operation == "init" then
    local failed = TacServices.activate(request)
    if failed ~= nil then
      return failed
    end
  end
  local owner = TacServices.owners[request.owner]
  if owner == nil or not owner:matches(request) then
    return TacAnswer.coded("Stale native owner.", "TY_NATIVE_OWNER")
  end
  if operation == "dispose" then
    TacServices.dispose_owner(owner.id)
    return TacAnswer.value(nil)
  end
  TacServices.executing = owner
  if operation == "init" then
    return TacAnswer.value(TacServices.describe(owner.metadata))
  end
  if not TacServices.present(request.name) then
    return TacAnswer.failure("Companion request has no member name.")
  end
  if operation == "call" or operation == "event" then
    TacTasks.allow_waiting(owner)
  end
  if operation == "event" then
    return TacServices.receive(owner, request.name, request.value)
  end
  return TacServices.use_member(owner, operation, request.name, request)
end

function TacServices.activate(request)
  local existing = TacServices.owners[request.owner]
  if existing ~= nil and existing:matches(request) then
    return nil
  end
  if existing ~= nil then
    TacServices.dispose_owner(request.owner)
  end
  local registered = TacServices.services[request.service]
  if registered == nil then
    return TacAnswer.coded("Unknown native service.", "TY_NATIVE_SERVICE")
  end
  if TacServices.owner_count >= TacLimits.MAX_OWNERS then
    return TacAnswer.failure("Too many native activations.")
  end
  local owner = TacOwner.new(request, TacServices.construct(registered.class), registered.metadata)
  TacServices.owners[owner.id] = owner
  TacServices.owner_count = TacServices.owner_count + 1
  TacServices.executing = owner
  TacServices.assign_properties(owner, request.props)
  return TacServices.start(owner)
end

function TacServices.start(owner)
  local ok = pcall(function()
    for _, name in ipairs(owner.metadata.starts or {}) do
      owner.instance[name](owner.instance)
    end
    TacServices.drain(owner)
  end)
  if ok then
    return nil
  end
  TacServices.dispose_owner(owner.id)
  return TacAnswer.failure("Native initializer failed.")
end

function TacServices.assign_properties(owner, properties)
  if type(properties) ~= "table" then
    return
  end
  local metadata = owner.metadata
  for key, value in pairs(properties) do
    local field = TacServices.lua_name(metadata, key)
    local required = metadata.required or {}
    local wanted = TacServices.contains(required, key) or TacServices.contains(required, field) or TacServices.contains(required, "*")
    if wanted and TacServices.contains(metadata.fields, key) then
      owner.instance[field] = value
    end
  end
end

function TacServices.describe(metadata)
  local listeners = {}
  for index, listener in ipairs(metadata.listeners or {}) do
    listeners[index] = {
      key = listener.method,
      events = listener.events,
      replay = listener.replay,
      order = listener.order,
      asynchronous = listener.asynchronous,
      condition = listener.condition,
      models = listener.models,
    }
  end
  return {
    fields = metadata.fields or {},
    methods = metadata.methods or {},
    listeners = listeners,
  }
end

function TacServices.receive(owner, name, value)
  local listener = nil
  for _, candidate in ipairs(owner.metadata.listeners or {}) do
    if candidate.method == name then
      listener = candidate
    end
  end
  if listener == nil then
    return TacAnswer.failure("Unknown companion listener: " .. name)
  end
  local ok, result = pcall(owner.instance[name], owner.instance, TacValues.model(listener.events[1], value))
  if not ok then
    return TacAnswer.failure("Native event listener failed.")
  end
  if result ~= nil and not listener.asynchronous then
    TacServices.republish(result, listener.returns)
  end
  return TacAnswer.value(nil)
end

function TacServices.use_member(owner, operation, name, request)
  local metadata = owner.metadata
  local is_field = TacServices.contains(metadata.fields, name)
  local is_method = TacServices.contains(metadata.methods, name)
  local lua_name = TacServices.lua_name(metadata, name)
  if operation == "get" and is_field then
    return TacAnswer.value(owner.instance[lua_name])
  end
  if operation == "set" and is_field then
    return TacServices.write(owner, name, lua_name, request.value)
  end
  if operation == "call" and is_method then
    return TacServices.call(owner, lua_name, request.args)
  end
  if not is_field and not is_method then
    return TacAnswer.failure("Unknown companion member: " .. name)
  end
  return TacAnswer.failure("Companion member does not support " .. operation .. ": " .. name)
end

function TacServices.write(owner, name, lua_name, value)
  if not TacServices.contains(owner.metadata.writable, name) then
    return TacAnswer.failure("Companion field is read-only: " .. name)
  end
  owner.instance[lua_name] = value
  return TacAnswer.value(nil)
end

function TacServices.call(owner, lua_name, arguments)
  if type(arguments) ~= "table" then
    arguments = {}
  end
  local method = owner.instance[lua_name]
  return TacAnswer.value(method(owner.instance, YonJson.arguments(arguments)))
end

function TacServices.publish(name, value, retain)
  local owner = TacServices.executing
  if owner == nil or owner.disposed then
    return TacServices.event_failure()
  end
  local ok, encoded = pcall(YonJson.encode, value)
  if not ok or #encoded > TacLimits.MAX_EVENT_BYTES or #owner.queue >= TacLimits.MAX_QUEUED_EVENTS then
    return TacServices.event_failure()
  end
  owner.queue[#owner.queue + 1] = { name, encoded, retain == true }
end

function TacServices.republish(result, returns)
  if type(returns) ~= "string" then
    return
  end
  local items = { result }
  if TacValues.model_name(result) == nil and type(result) == "table" and YonJson.is_list(result) then
    items = table.move(result, 1, math.min(#result, TacLimits.MAX_RETURNED_EVENTS), 1, {})
  end
  for _, item in ipairs(items) do
    TacServices.publish(returns, item, false)
  end
end

function TacServices.drain(owner)
  while not owner.disposed and #owner.queue > 0 do
    local event = table.remove(owner.queue, 1)
    TacServices.send_event(owner, event[1], event[2], event[3])
  end
end

function TacServices.send_event(owner, name, encoded, retain)
  TachyonHost.emit("{\"route\":" .. YonJson.text(owner.route) .. ",\"service\":" .. YonJson.text(owner.service)
    .. ",\"owner\":" .. YonJson.text(owner.id) .. ",\"name\":" .. YonJson.text(name)
    .. ",\"value\":" .. encoded .. ",\"retain\":" .. tostring(retain) .. "}")
end

function TacServices.event_failure()
  warn("Tachyon native event delivery failed.")
  return nil
end

function TacServices.dispose_owner(id)
  TacTasks.cancel(id)
  local owner = TacServices.owners[id]
  if owner == nil then
    return
  end
  TacServices.owners[id] = nil
  TacServices.owner_count = TacServices.owner_count - 1
  if TacServices.executing ~= nil and TacServices.executing.id == id then
    TacServices.executing = nil
  end
  local ok = pcall(owner.dispose, owner)
  if not ok then
    TacServices.event_failure()
  end
end

function TacServices.dispose_all()
  TacTasks.cancel(nil)
  local active = TacServices.owners
  TacServices.owners = {}
  TacServices.owner_count = 0
  TacServices.executing = nil
  for _, owner in pairs(active) do
    pcall(owner.dispose, owner)
  end
end
