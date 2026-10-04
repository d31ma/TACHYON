ApplicationEventPublisher = {}

function ApplicationEventPublisher:publish_event(value, retain)
  local model = type(value) == "table" and getmetatable(value) or nil
  local name = type(model) == "table" and rawget(model, "__name") or nil
  if name == nil then
    io.stderr:write("Yon local event publication failed.\n")
    return value
  end
  YonEvents.publish(name, value, retain == true)
  return value
end

YonEvents = {
  MAX_EVENT_BYTES = 65536,
  MAX_QUEUED_EVENTS = 128,
  MAX_LISTENERS = 1024,
  MAX_DELIVERIES = 4096,
  MAX_REPUBLISHED_EVENTS = 32,
  active = nil,
  topics = {},
}
YonEvents.__index = YonEvents

YonEvents.RESOURCES = { application_event_publisher = ApplicationEventPublisher }

function YonEvents.members(class)
  return {
    __index = function(_, key)
      local value = class[key]
      if value == nil then
        return YonEvents.RESOURCES[key]
      end
      return value
    end,
  }
end

function YonEvents.new(only)
  if YonEvents.active ~= nil then
    error("Nested local event invocation", 0)
  end
  local bus = setmetatable({
    only = YonEvents.keys(only),
    outbound = {},
    queue = {},
    retained = {},
    listeners = {},
    destroyers = {},
    created = {},
    closed = false,
    deliveries = 0,
  }, YonEvents)
  YonEvents.active = bus
  return bus
end

function YonEvents.keys(only)
  if only == nil then
    return nil
  end
  local keys = {}
  for _, key in ipairs(only) do
    keys[key] = true
  end
  return keys
end

function YonEvents.publish(name, value, retain)
  local ok = pcall(function()
    local scope = YonEvents.active
    if scope == nil or scope.closed then
      error("Local event owner is disposed", 0)
    end
    scope:enqueue(name, YonJson.encode(value), retain)
  end)
  if not ok then
    io.stderr:write("Yon local event publication failed.\n")
  end
end

function YonEvents.track(cache, index)
  local scope = YonEvents.active
  if scope ~= nil and cache == scope.cache then
    scope.created[#scope.created + 1] = index
  end
end

function YonEvents:enqueue(name, encoded, retain)
  if #encoded > YonEvents.MAX_EVENT_BYTES or #self.queue >= YonEvents.MAX_QUEUED_EVENTS then
    error("Local event capacity exceeded", 0)
  end
  if retain then
    self.retained[name] = encoded
  end
  self.queue[#self.queue + 1] = { name, encoded }
  if YonEvents.topics[name] then
    self.outbound[#self.outbound + 1] = { name, encoded }
  end
end

function YonEvents:receive(name, encoded)
  self.queue[#self.queue + 1] = { name, encoded }
end

function YonEvents:listen(options, callback, replay, key)
  if self.only ~= nil and not self.only[key] then
    return
  end
  options = YonJson.decode(options)
  for _, name in ipairs(options.events) do
    if #self.listeners >= YonEvents.MAX_LISTENERS then
      error("Local listener capacity exceeded", 0)
    end
    self.listeners[#self.listeners + 1] = {
      name = name,
      callback = callback,
      order = math.tointeger(options.order) or 0,
      condition = options.condition,
      models = options.models,
      returns = options.returns,
      sequence = #self.listeners,
    }
    if replay and self.retained[name] ~= nil then
      self.queue[#self.queue + 1] = { name, self.retained[name] }
    end
  end
  table.sort(self.listeners, YonEvents.earlier)
end

function YonEvents.earlier(first, second)
  if first.order ~= second.order then
    return first.order < second.order
  end
  return first.sequence < second.sequence
end

function YonEvents:start(cache, starts)
  for _, index in ipairs(self.created) do
    local method = starts[index]
    if method ~= nil then
      cache[index][method](cache[index])
    end
  end
end

function YonEvents:stop_later(cache, stops)
  for _, index in ipairs(self.created) do
    local method = stops[index]
    if method ~= nil then
      self.destroyers[#self.destroyers + 1] = function()
        cache[index][method](cache[index])
      end
    end
  end
end

function YonEvents:drain()
  while #self.queue > 0 do
    local event = table.remove(self.queue, 1)
    self.deliveries = self.deliveries + 1
    if self.deliveries > YonEvents.MAX_DELIVERIES then
      self.queue = {}
      io.stderr:write("Yon local event delivery bound exceeded.\n")
      return
    end
    for _, listener in ipairs(self.listeners) do
      if listener.name == event[1] then
        self:deliver(listener, event[1], event[2])
      end
    end
  end
  self:send_topics()
end

function YonEvents:send_topics()
  for _, event in ipairs(self.outbound) do
    YonTopics.send(event[1], event[2])
  end
  self.outbound = {}
end

function YonEvents:close()
  if self.closed then
    return
  end
  self.closed = true
  self.queue = {}
  self.listeners = {}
  for index = #self.destroyers, 1, -1 do
    local ok = pcall(self.destroyers[index])
    if not ok then
      io.stderr:write("Yon local lifecycle cleanup failed.\n")
    end
  end
  self.retained = {}
  YonEvents.active = nil
end

function YonEvents:deliver(listener, name, encoded)
  local ok = pcall(function()
    local value = YonJson.decode(encoded)
    if listener.models ~= nil and not YonEventModels.fits(listener.models, name, value) then
      io.stderr:write("Yon local event payload does not fit the listener model.\n")
      return
    end
    if listener.condition ~= nil and not YonEventConditions.matches(listener.condition, value, name) then
      return
    end
    self:republish(listener.callback(value), listener.returns)
  end)
  if not ok then
    io.stderr:write("Yon local event listener failed.\n")
  end
end

function YonEvents:republish(result, returns)
  if returns == nil or result == nil then
    return
  end
  local items = { result }
  if getmetatable(result) == nil and type(result) == "table" and YonJson.is_list(result) then
    items = table.move(result, 1, math.min(#result, YonEvents.MAX_REPUBLISHED_EVENTS), 1, {})
  end
  for _, item in ipairs(items) do
    YonEvents.publish(returns, item, false)
  end
end

YonEventModels = {}

function YonEventModels.fits(models, shape, value)
  if shape:sub(1, 1) == "?" then
    return value == nil or YonEventModels.fits(models, shape:sub(2), value)
  end
  if shape:sub(1, 2) == "[]" then
    return YonEventModels.fits_list(models, shape:sub(3), value)
  end
  local scalar = YonEventModels.SCALARS[shape]
  if scalar ~= nil then
    return scalar(value)
  end
  return YonEventModels.fits_model(models, models[shape], value)
end

function YonEventModels.fits_list(models, shape, value)
  if type(value) ~= "table" or not YonJson.is_list(value) then
    return false
  end
  for _, item in ipairs(value) do
    if not YonEventModels.fits(models, shape, item) then
      return false
    end
  end
  return true
end

function YonEventModels.fits_model(models, model, value)
  if model == nil or type(value) ~= "table" then
    return false
  end
  local fields = model.fields
  if not model.open then
    for key in pairs(value) do
      if fields[key] == nil then
        return false
      end
    end
  end
  for name, field in pairs(fields) do
    local required = field:sub(1, 1) == "!"
    local shape = required and field:sub(2) or field
    if value[name] == nil then
      if required then
        return false
      end
    elseif not YonEventModels.fits(models, shape, value[name]) then
      return false
    end
  end
  return true
end

YonEventModels.SCALARS = {
  any = function()
    return true
  end,
  string = function(value)
    return type(value) == "string"
  end,
  integer = function(value)
    return math.type(value) == "integer"
  end,
  number = function(value)
    return type(value) == "number"
  end,
  boolean = function(value)
    return type(value) == "boolean"
  end,
}

YonEventConditions = {}

function YonEventConditions.matches(node, value, event)
  local ok, matched = pcall(YonEventConditions.test, node, value, event)
  return ok and matched
end

function YonEventConditions.test(node, value, event)
  local kind = node.k
  if kind == "and" then
    return YonEventConditions.test(node.l, value, event) and YonEventConditions.test(node.r, value, event)
  elseif kind == "or" then
    return YonEventConditions.test(node.l, value, event) or YonEventConditions.test(node.r, value, event)
  elseif kind == "not" then
    return not YonEventConditions.test(node.e, value, event)
  elseif kind == "truthy" then
    return YonEventConditions.truthy(YonEventConditions.operand(node.e, value, event))
  elseif kind == "cmp" then
    return YonEventConditions.compare(node, value, event)
  end
  return false
end

function YonEventConditions.operand(node, value, event)
  if node.k == "lit" then
    return node.v
  end
  if node.k == "event" then
    return event
  end
  local current = value
  for _, segment in ipairs(node.path or {}) do
    if type(current) ~= "table" then
      return nil
    end
    current = current[segment]
  end
  return current
end

function YonEventConditions.truthy(value)
  return not (value == nil or value == false or value == 0 or value == "" or value ~= value)
end

function YonEventConditions.compare(node, value, event)
  local left = YonEventConditions.operand(node.l, value, event)
  local right = YonEventConditions.operand(node.r, value, event)
  local numbers = type(left) == "number" and type(right) == "number"
  local strings = type(left) == "string" and type(right) == "string"
  if node.op == "eq" or node.op == "ne" then
    local equal = left == right and (numbers or type(left) == type(right))
    return (node.op == "eq") == equal
  end
  if not numbers and not strings then
    return false
  end
  local order = YonEventConditions.ORDERS[node.op] or YonEventConditions.ORDERS.ge
  return order(left, right)
end

YonEventConditions.ORDERS = {
  lt = function(left, right)
    return left < right
  end,
  le = function(left, right)
    return left <= right
  end,
  gt = function(left, right)
    return left > right
  end,
  ge = function(left, right)
    return left >= right
  end,
}
