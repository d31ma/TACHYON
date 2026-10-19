BehaviorError = {}
BehaviorError.__index = BehaviorError
BehaviorError.__tostring = function(failure)
  return failure.message
end

function BehaviorError.raise(behavior_code, message, path)
  error(setmetatable({ behavior_code = behavior_code, message = message, path = path }, BehaviorError), 0)
end

function BehaviorError.is(failure)
  return type(failure) == "table" and getmetatable(failure) == BehaviorError
end

BehaviorSignal = {}
BehaviorSignal.__index = BehaviorSignal

function BehaviorSignal.new()
  return setmetatable({ cancelled = false }, BehaviorSignal)
end

function BehaviorSignal:abort()
  self.cancelled = true
end

function BehaviorSignal:aborted()
  return self.cancelled
end

BehaviorClock = {
  POSIX_SLEEP = "sleep %d.%03d",
  WINDOWS_SLEEP = "powershell -NoProfile -NonInteractive -Command Start-Sleep -Milliseconds %d",
  WINDOWS_SEPARATOR = "\\",
  host = BehaviorHostClock,
  started_seconds = os and os.time(),
  slept_ms = 0,
}

function BehaviorClock.now_ms()
  if BehaviorClock.host ~= nil then
    return BehaviorClock.host.now_ms()
  end
  return os.time() * 1000
end

function BehaviorClock.monotonic_ms()
  if BehaviorClock.host ~= nil then
    return BehaviorClock.host.monotonic_ms()
  end
  local measured = math.floor(os.clock() * 1000) + BehaviorClock.sleep_not_in_clock_ms()
  local whole_seconds_passed = os.difftime(os.time(), BehaviorClock.started_seconds) - 1
  return math.max(measured, math.floor(whole_seconds_passed * 1000))
end

-- os.clock() is C clock(): processor time on POSIX, but wall time on Windows,
-- where it already includes the time spent sleeping.
function BehaviorClock.clock_counts_sleep()
  return package.config:sub(1, 1) == BehaviorClock.WINDOWS_SEPARATOR
end

function BehaviorClock.sleep_not_in_clock_ms()
  if BehaviorClock.clock_counts_sleep() then
    return 0
  end
  return BehaviorClock.slept_ms
end

function BehaviorClock.wait(milliseconds)
  if BehaviorClock.host ~= nil then
    return BehaviorClock.host.wait(milliseconds)
  end
  local whole = math.floor(milliseconds)
  if whole <= 0 then
    return
  end
  if os.execute(BehaviorClock.sleep_command(whole, package.config:sub(1, 1))) then
    BehaviorClock.slept_ms = BehaviorClock.slept_ms + whole
  end
end

function BehaviorClock.sleep_command(milliseconds, separator)
  if separator == BehaviorClock.WINDOWS_SEPARATOR then
    return string.format(BehaviorClock.WINDOWS_SLEEP, milliseconds)
  end
  return string.format(BehaviorClock.POSIX_SLEEP, milliseconds // 1000, milliseconds % 1000)
end

BehaviorCancellation = {}
BehaviorCancellation.__index = BehaviorCancellation

function BehaviorCancellation.new(context, owner, until_ms)
  return setmetatable({ context = context, owner = owner, until_ms = until_ms }, BehaviorCancellation)
end

function BehaviorCancellation:aborted()
  return self.context:aborted() or self.owner:aborted() or BehaviorClock.monotonic_ms() >= self.until_ms
end

BehaviorExecution = {}
BehaviorExecution.__index = BehaviorExecution

function BehaviorExecution.new(partition, environment, signal, owner, timeout_ms)
  local until_ms = BehaviorClock.monotonic_ms() + timeout_ms
  return setmetatable({
    partition = partition,
    environment = environment,
    context_signal = signal,
    owner = owner,
    deadline = BehaviorClock.now_ms() + timeout_ms,
    until_ms = until_ms,
    signal = BehaviorCancellation.new(signal, owner, until_ms),
    policy = {},
    expires_at = 0,
    public_read = false,
    revalidate = false,
  }, BehaviorExecution)
end

function BehaviorExecution:check()
  if self.context_signal:aborted() or self.owner:aborted() then
    BehaviorError.raise("cancelled", "Behavior operation was cancelled.")
  end
  if BehaviorClock.monotonic_ms() >= self.until_ms then
    BehaviorError.raise("timeout", "Behavior operation exceeded its deadline.")
  end
end

BehaviorBoundary = {
  MAX_SAFE_INTEGER = 9007199254740991,
  UNSAFE_FIELDS = { __proto__ = true, prototype = true, constructor = true, behaviors = true },
  ADAPTER_NAME = "^[%a_$][%w_$]*$",
  MAX_ADAPTER_NAME = 128,
  GATEWAY_FAILURES = { invalid = true, timeout = true, capacity = true, adapter = true },
}

function BehaviorBoundary.invalid(message)
  BehaviorError.raise("invalid", message)
end

function BehaviorBoundary.short_name(type_table)
  if type(type_table) == "table" then
    return rawget(type_table, "__name") or ""
  end
  return tostring(type_table)
end

function BehaviorBoundary.adapter_name(name)
  return type(name) == "string" and #name <= BehaviorBoundary.MAX_ADAPTER_NAME and name:match(BehaviorBoundary.ADAPTER_NAME) ~= nil
end

function BehaviorBoundary.keys(value, allowed)
  if type(value) ~= "table" then
    BehaviorBoundary.invalid("Unknown or invalid behavior options.")
  end
  local known = {}
  for _, name in ipairs(allowed) do
    known[name] = true
  end
  for key in pairs(value) do
    if not known[key] then
      BehaviorBoundary.invalid("Unknown or invalid behavior options.")
    end
  end
  return value
end

function BehaviorBoundary.copy(value, limit)
  limit = limit or 1048576
  local counter = { nodes = 0, characters = 0 }
  local result = BehaviorBoundary.walk(value, 0, counter, limit)
  if #BehaviorBoundary.encode(result) > limit then
    BehaviorBoundary.invalid("Behavior JSON exceeds byte bounds.")
  end
  return result
end

function BehaviorBoundary.encode(value)
  local ok, text = pcall(YonJson.encode, value)
  if not ok then
    BehaviorBoundary.invalid("Behavior JSON is not valid UTF-8 JSON.")
  end
  return text
end

function BehaviorBoundary.walk(value, depth, counter, limit)
  counter.nodes = counter.nodes + 1
  if counter.nodes > 10000 or depth > 32 then
    BehaviorBoundary.invalid("Behavior JSON exceeds structural bounds.")
  end
  local kind = type(value)
  if value == nil or value == YonNull or kind == "boolean" then
    return value
  end
  if kind == "number" then
    return BehaviorBoundary.number(value)
  end
  if kind == "string" then
    return BehaviorBoundary.text(value, counter, limit)
  end
  if kind == "table" then
    return BehaviorBoundary.walk_table(value, depth, counter, limit)
  end
  BehaviorBoundary.invalid("Behavior values must be finite JSON data.")
end

function BehaviorBoundary.number(value)
  local finite = value == value and value ~= math.huge and value ~= -math.huge
  local safe = math.type(value) ~= "integer" or math.abs(value) <= BehaviorBoundary.MAX_SAFE_INTEGER
  if not finite or not safe then
    BehaviorBoundary.invalid("Behavior values must be finite JSON data.")
  end
  return value
end

function BehaviorBoundary.text(value, counter, limit)
  if utf8.len(value) == nil then
    BehaviorBoundary.invalid("Behavior JSON is not valid UTF-8 JSON.")
  end
  counter.characters = counter.characters + #value
  if counter.characters > limit then
    BehaviorBoundary.invalid("Behavior JSON exceeds byte bounds.")
  end
  return value
end

function BehaviorBoundary.walk_table(value, depth, counter, limit)
  local fields = YonModels.fields_of(value)
  if fields == nil and getmetatable(value) ~= YonJson.OBJECT and YonJson.is_list(value) then
    return BehaviorBoundary.walk_list(value, depth, counter, limit)
  end
  return BehaviorBoundary.walk_map(value, depth, counter, limit)
end

function BehaviorBoundary.walk_list(items, depth, counter, limit)
  if #items > 10000 then
    BehaviorBoundary.invalid("Behavior collection exceeds bounds.")
  end
  local result = {}
  for index = 1, #items do
    result[index] = BehaviorBoundary.walk(items[index], depth + 1, counter, limit)
  end
  return result
end

function BehaviorBoundary.walk_map(items, depth, counter, limit)
  local result = setmetatable({}, YonJson.OBJECT)
  local count = 0
  for key, item in pairs(items) do
    count = count + 1
    local unsafe = type(key) ~= "string" or BehaviorBoundary.UNSAFE_FIELDS[key] or (key == "tac" and depth ~= 0)
    if unsafe or count > 10000 then
      BehaviorBoundary.invalid("Unsafe behavior JSON field.")
    end
    BehaviorBoundary.walk(key, depth + 1, counter, limit)
    result[key] = BehaviorBoundary.walk(item, depth + 1, counter, limit)
  end
  return result
end

function BehaviorBoundary.page(value, policy)
  value = BehaviorBoundary.keys(value, { "page", "size", "offset", "sort" })
  BehaviorBoundary.copy(value, 4096)
  local page = value.page
  if page == nil then
    page = 0
  end
  local size = value.size
  if size == nil then
    size = policy.defaultSize
  end
  local bounded = math.type(page) == "integer" and math.type(size) == "integer" and page >= 0 and page <= 1000000
    and size >= 1 and size <= policy.maxSize and page * size <= 1000000
  if not bounded then
    BehaviorBoundary.invalid("Invalid behavior pagination bounds.")
  end
  if value.offset ~= nil and value.offset ~= page * size then
    BehaviorBoundary.invalid("Invalid behavior pagination offset.")
  end
  return { page = page, size = size, offset = page * size, sort = BehaviorBoundary.sort(value.sort or {}, policy.sort) }
end

function BehaviorBoundary.sort(items, allowed)
  if type(items) ~= "table" or #items > 16 then
    BehaviorBoundary.invalid("Invalid behavior sort.")
  end
  local permitted = {}
  for _, field in ipairs(allowed or {}) do
    permitted[field] = true
  end
  local seen = {}
  local result = {}
  for index, item in ipairs(items) do
    local field, direction, extra = BehaviorBoundary.sort_parts(item)
    direction = direction or "asc"
    local valid = extra == nil and permitted[field] and (direction == "asc" or direction == "desc") and not seen[field]
    if not valid then
      BehaviorBoundary.invalid("Invalid behavior sort.")
    end
    seen[field] = true
    result[index] = field .. ":" .. direction
  end
  return result
end

function BehaviorBoundary.sort_parts(item)
  if type(item) ~= "string" then
    BehaviorBoundary.invalid("Invalid behavior sort.")
  end
  local parts = {}
  for part in (item .. ":"):gmatch("([^:]*):") do
    parts[#parts + 1] = part
  end
  return parts[1], parts[2], parts[3]
end

function BehaviorBoundary.adapter(action, arguments, execution)
  execution:check()
  local results = table.pack(pcall(action, table.unpack(arguments, 1, arguments.n or #arguments)))
  if not results[1] then
    if BehaviorError.is(results[2]) then
      error(results[2], 0)
    end
    BehaviorError.raise("adapter", "Behavior adapter failed.")
  end
  execution:check()
  return results[2]
end

function BehaviorBoundary.map_value(policy, arguments)
  local value = arguments[policy.argument + 1]
  if type(value) ~= "table" then
    BehaviorBoundary.invalid("A behavior mapping reads one JSON object argument.")
  end
  local output = setmetatable({}, YonJson.OBJECT)
  for _, field in ipairs(policy.fields) do
    output[field.target] = BehaviorBoundary.mapped_field(field, value)
  end
  return BehaviorBoundary.copy(output)
end

function BehaviorBoundary.mapped_field(field, value)
  local mapped = BehaviorBoundary.field_value(field, value)
  if mapped == nil then
    return YonNull
  end
  return mapped
end

function BehaviorBoundary.field_value(field, value)
  if field.source == nil then
    return field.constant
  end
  local current = value
  for name in field.source:gmatch("[^.]+") do
    if type(current) ~= "table" then
      return nil
    end
    current = current[name]
  end
  if field.fields ~= nil then
    return BehaviorBoundary.map_shape(current, field.fields)
  end
  return current
end

function BehaviorBoundary.map_shape(value, fields)
  if type(value) ~= "table" then
    return value
  end
  if getmetatable(value) ~= YonJson.OBJECT and YonJson.is_list(value) then
    local output = {}
    for index, item in ipairs(value) do
      output[index] = BehaviorBoundary.map_shape(item, fields)
    end
    return output
  end
  return BehaviorBoundary.map_value({ argument = 0, fields = fields }, { value })
end

function BehaviorBoundary.gateway(key, arguments)
  if YonGateway == nil then
    BehaviorError.raise("adapter", "Gateway call failed.")
  end
  local count = arguments.n or #arguments
  local list = {}
  for index = 1, count do
    local value = arguments[index]
    if value == nil then
      value = YonNull
    end
    list[index] = value
  end
  local answer = YonGateway.call(key, list)
  if type(answer) ~= "table" or type(answer.error) == "string" then
    local kind = type(answer) == "table" and answer.error or "adapter"
    if not BehaviorBoundary.GATEWAY_FAILURES[kind] then
      kind = "adapter"
    end
    BehaviorError.raise(kind, "Gateway call failed.")
  end
  return answer.result
end

function BehaviorBoundary.retryable(failure)
  return not BehaviorError.is(failure) or failure.behavior_code == "adapter"
end

function BehaviorBoundary.backoff(policy, attempt)
  local delay = math.min(policy.delayMs * (policy.multiplier ^ attempt), policy.maxDelayMs)
  return math.floor(delay + math.random() * policy.jitterMs + 0.5)
end

function BehaviorBoundary.entity_id(id)
  local text = type(id) == "string" and #id > 0 and #id <= 256
  local whole = math.type(id) == "integer" and math.abs(id) <= BehaviorBoundary.MAX_SAFE_INTEGER
  if not text and not whole then
    BehaviorBoundary.invalid("Invalid entity identifier.")
  end
  return BehaviorBoundary.copy(id, 1024)
end

function BehaviorBoundary.callable(value)
  if type(value) == "function" then
    return true
  end
  local meta = getmetatable(value)
  return type(meta) == "table" and type(meta.__call) == "function"
end

BehaviorExchange = {
  PATH_VALUE = "^[%w._~-]+$",
  MAX_PATH_VALUE = 256,
}

function BehaviorExchange.scalar(value)
  return type(value) == "string" or type(value) == "number" or type(value) == "boolean"
end

function BehaviorExchange.path(policy, arguments)
  local path = policy.path
  for name, index in pairs(policy.pathVariables or {}) do
    local value = arguments[index + 1]
    local text = value
    if type(value) == "number" or type(value) == "boolean" then
      text = tostring(value)
    end
    local safe = type(text) == "string" and #text <= BehaviorExchange.MAX_PATH_VALUE
      and text:match(BehaviorExchange.PATH_VALUE) ~= nil and text ~= "." and text ~= ".."
    if not safe then
      BehaviorBoundary.invalid("A path variable may use only letters, digits and - _ . ~")
    end
    path = path:gsub("{" .. name:gsub("%p", "%%%0") .. "}", text)
  end
  return path
end

function BehaviorExchange.present(value, message)
  if value == nil or value == YonNull then
    BehaviorBoundary.invalid(message)
  end
  return value
end

function BehaviorExchange.request(policy, arguments)
  local count = arguments.n or #arguments
  local pick = function(index)
    if index >= count then
      BehaviorBoundary.invalid("A request argument is missing.")
    end
    return arguments[index + 1]
  end
  local descriptor = { query = {}, headers = {} }
  for name, index in pairs(policy.query or {}) do
    descriptor.query[name] = BehaviorExchange.present(pick(index), "Invalid HTTP query.")
  end
  for name, index in pairs(policy.headers or {}) do
    descriptor.headers[name] = BehaviorExchange.present(pick(index), "Invalid HTTP header.")
  end
  if policy.body ~= nil then
    descriptor.body = pick(policy.body)
  end
  return descriptor
end

BehaviorOpenTransactions = { open = setmetatable({}, { __mode = "k" }) }

function BehaviorOpenTransactions.of(adapter)
  return (BehaviorOpenTransactions.open[coroutine.running()] or {})[adapter]
end

function BehaviorOpenTransactions.within(adapter, identifier, action)
  local task = coroutine.running()
  local outer = BehaviorOpenTransactions.open[task]
  local inner = {}
  for key, value in pairs(outer or {}) do
    inner[key] = value
  end
  inner[adapter] = identifier
  BehaviorOpenTransactions.open[task] = inner
  local results = table.pack(pcall(action))
  BehaviorOpenTransactions.open[task] = outer
  if not results[1] then
    error(results[2], 0)
  end
  return results[2]
end

BehaviorContext = {
  ADAPTER_GROUPS = { "caches", "http", "entities" },
  ADAPTER_KEYS = { "caches", "http", "entities", "authorize", "authorizeEntity" },
  READS = { GET = true, HEAD = true },
  TRANSPORT_HEADERS = { host = true, connection = true, ["content-length"] = true, ["transfer-encoding"] = true, cookie = true },
  HEADER_NAME = "^[!#$%%&'*+.^_`|~%w-]+$",
  MAX_RESPONSE_BYTES = 1048576,
  MAX_REVALIDATIONS = 256,
  RESERVED_NAMES = { __proto__ = true, prototype = true, constructor = true, behaviors = true, tac = true },
  RESULT_KINDS = { find = "list", count = "count", exists = "boolean" },
}
BehaviorContext.__index = BehaviorContext

function BehaviorContext.new(records, options)
  options = BehaviorBoundary.keys(options, { "partition", "signal", "timeoutMs", "profiles", "properties" })
  local partition = options.partition
  local signal = options.signal
  local timeout = options.timeoutMs or 10000
  local valid = type(partition) == "string" and partition:find("%S") ~= nil and #partition <= 256
    and getmetatable(signal) == BehaviorSignal and math.type(timeout) == "integer" and timeout >= 1 and timeout <= 60000
  if not valid then
    BehaviorBoundary.invalid("Explicit bounded partition, BehaviorSignal and timeout are required.")
  end
  return setmetatable({
    records = records,
    partition = partition,
    owner = signal,
    signal = BehaviorSignal.new(),
    timeout_ms = timeout,
    adapters = {},
    slots = {},
    revalidations = {},
    active = 0,
  }, BehaviorContext)
end

function BehaviorContext:configure(factory)
  local execution = self:start()
  local ok, failure = pcall(function()
    local adapters = BehaviorBoundary.keys(BehaviorBoundary.adapter(factory, { execution, n = 1 }, execution), BehaviorContext.ADAPTER_KEYS)
    for _, group in ipairs(BehaviorContext.ADAPTER_GROUPS) do
      adapters[group] = BehaviorContext.checked_collection(adapters[group] or {})
    end
    for _, key in ipairs({ "authorize", "authorizeEntity" }) do
      if adapters[key] ~= nil and not BehaviorBoundary.callable(adapters[key]) then
        BehaviorBoundary.invalid("Authorize must be callable.")
      end
    end
    self.adapters = adapters
  end)
  self.active = self.active - 1
  if not ok then
    error(failure, 0)
  end
end

function BehaviorContext.checked_collection(collection)
  if type(collection) ~= "table" then
    BehaviorBoundary.invalid("Invalid adapter collection.")
  end
  local count = 0
  for name in pairs(collection) do
    count = count + 1
    if count > 128 or not BehaviorBoundary.adapter_name(name) or BehaviorContext.RESERVED_NAMES[name] then
      BehaviorBoundary.invalid("Invalid adapter name.")
    end
  end
  return collection
end

function BehaviorContext:dispose()
  self.signal:abort()
end

function BehaviorContext:run(action)
  local execution = self:start()
  local results = table.pack(pcall(action, execution))
  self.active = self.active - 1
  if not results[1] then
    error(results[2], 0)
  end
  return results[2]
end

function BehaviorContext:invoke(instance, type_name, method, arguments, original)
  local record = self:record_of(instance, type_name, method)
  local metadata = record.methods[method]
  return self:run(function(execution)
    arguments = self:checked_arguments(arguments, metadata)
    if metadata.query ~= nil then
      return self:query(metadata.query, arguments, execution)
    end
    if metadata.cachePut ~= nil or metadata.cacheEvict ~= nil then
      return self:mutate(record, metadata, arguments, original, execution)
    end
    return self:read_through(record, method, metadata, arguments, original, execution)
  end)
end

function BehaviorContext:read_through(record, method, metadata, arguments, original, execution)
  local key = nil
  local policy = record.cache
  if metadata.cacheable ~= nil then
    key = self:cache_key(record, method, arguments, execution, "cache:read")
    local hit = self:cached(policy, key, execution)
    if hit ~= nil then
      return hit.value
    end
    execution.public_read = true
    execution.revalidate = self:take_revalidation(key)
  end
  execution:check()
  local result = BehaviorBoundary.copy(self:resilient(record, method, metadata, arguments, original, execution))
  if key ~= nil then
    self:store(policy, key, result, execution)
  end
  execution:check()
  return result
end

function BehaviorContext:evict(instance, method, arguments)
  local record = self:record_of(instance, nil, method)
  if record.methods[method].cacheable == nil then
    BehaviorBoundary.invalid("Eviction requires a cacheable method.")
  end
  return self:run(function(execution)
    arguments = self:checked_arguments(arguments, record.methods[method])
    local key = self:cache_key(record, method, arguments, execution, "cache:evict")
    BehaviorBoundary.adapter(self:adapter("caches", record.cache.name, "delete"), { key, execution, n = 2 }, execution)
    return nil
  end)
end

function BehaviorContext:repository(entity)
  local record = self.records[BehaviorBoundary.short_name(entity)]
  if record == nil or record.entity == nil then
    BehaviorBoundary.invalid("Store requires source-local Entity/Table.")
  end
  return BehaviorRepository.new(self, record)
end

function BehaviorContext:entity_operation(record, operation, value)
  local owned = type(record) == "table" and self.records[record.name] == record
  if not owned then
    BehaviorBoundary.invalid("Entity belongs to another module.")
  end
  local known = record.entity ~= nil and record.table ~= nil and (operation == "find" or operation == "save" or operation == "delete")
  if not known then
    BehaviorBoundary.invalid("Unknown entity operation.")
  end
  value = self:entity_value(record, operation, value)
  return self:run(function(execution)
    local field = ({ find = "page", save = "value", delete = "id" })[operation]
    local command = { operation = operation, entity = record.name, table = record.table, idField = record.entity.id }
    command[field] = BehaviorBoundary.copy(value)
    command.transaction = BehaviorOpenTransactions.of(record.entity.adapter)
    local action = self:adapter("entities", record.entity.adapter, "execute")
    return BehaviorBoundary.copy(BehaviorBoundary.adapter(action, { command, execution, n = 2 }, execution))
  end)
end

function BehaviorContext:start()
  local execution = BehaviorExecution.new(self.partition, Behaviors.environment, self.signal, self.owner, self.timeout_ms)
  execution:check()
  if self.active >= 64 then
    BehaviorBoundary.invalid("Behavior concurrency exceeds bounds.")
  end
  self.active = self.active + 1
  return execution
end

function BehaviorContext:adapter(group, name, operation)
  local found = (self.adapters[group] or {})[name]
  if found == nil then
    found = BehaviorStorage.adapter(group, name)
  end
  local action = type(found) == "table" and found[operation] or nil
  if type(action) ~= "function" then
    BehaviorBoundary.invalid("Required behavior adapter is missing.")
  end
  return function(...)
    return action(found, ...)
  end
end

function BehaviorContext:builtin(group, name)
  return (self.adapters[group] or {})[name] == nil and BehaviorStorage.adapter(group, name) ~= nil
end

function BehaviorContext:checked_arguments(arguments, metadata)
  local count = arguments.n or #arguments
  local checked = table.move(arguments, 1, count, 1, { n = count })
  if metadata.valid ~= nil then
    local index = metadata.valid.argument + 1
    checked[index] = BehaviorValidation.guard(metadata.valid.schema, checked[index])
  end
  if count > 32 then
    BehaviorBoundary.invalid("Too many behavior arguments.")
  end
  if metadata.pageable ~= nil then
    local index = metadata.pageable.argument + 1
    if index > count or type(checked[index]) ~= "table" then
      BehaviorBoundary.invalid("Pagination requires an argument map.")
    end
    checked[index] = BehaviorBoundary.page(checked[index], metadata.pageable)
  end
  return checked
end

function BehaviorContext:record_of(instance, type_name, method)
  local record = type_name and self.records[type_name] or nil
  if record == nil then
    record = BehaviorContext.record_with(self.records, method)
  end
  local owned = type(instance) == "table" and instance.behaviors == self and record ~= nil and record.methods[method] ~= nil
  if not owned then
    BehaviorBoundary.invalid("Behavior instance requires its source-local context.")
  end
  return record
end

function BehaviorContext.record_with(records, method)
  for _, record in pairs(records) do
    if record.methods ~= nil and record.methods[method] ~= nil then
      return record
    end
  end
  return nil
end

function BehaviorContext:cache_key(record, method, arguments, execution, operation)
  local selected = {}
  for index, position in ipairs(record.methods[method].cacheable.keyArgs) do
    if position >= (arguments.n or #arguments) then
      BehaviorBoundary.invalid("Cache key argument is missing.")
    end
    selected[index] = arguments[position + 1]
  end
  selected = BehaviorBoundary.copy(selected, 4096)
  local call = { operation = operation, cache = record.cache.name, type = record.name, method = method, keyArguments = BehaviorBoundary.copy(selected) }
  local authorize = self.adapters.authorize
  local built_in = authorize == nil and self:builtin("caches", record.cache.name)
  local allowed = built_in or (BehaviorBoundary.callable(authorize) and BehaviorBoundary.adapter(authorize, { call, execution, n = 2 }, execution) == true)
  if not allowed then
    BehaviorError.raise("unauthorized", "Behavior cache access denied.")
  end
  return BehaviorBoundary.encode(BehaviorBoundary.copy({ self.partition, record.cache.name, record.module, record.name, method, selected }, 4096))
end

function BehaviorContext:cached(policy, key, execution)
  local hit = BehaviorBoundary.adapter(self:adapter("caches", policy.name, "get"), { key, execution, n = 2 }, execution)
  hit = BehaviorBoundary.keys(hit, { "hit", "value", "expiresAt" })
  if hit.hit ~= true and hit.hit ~= false then
    BehaviorBoundary.invalid("Invalid cache response.")
  end
  if not hit.hit then
    return nil
  end
  if math.type(hit.expiresAt) ~= "integer" then
    BehaviorBoundary.invalid("Invalid cache expiry/value.")
  end
  local now = BehaviorClock.now_ms()
  if hit.expiresAt > now + policy.ttlMs then
    BehaviorBoundary.invalid("Cache expiry exceeds its policy.")
  end
  if hit.expiresAt <= now then
    return nil
  end
  local value = BehaviorBoundary.copy(hit.value, policy.maxBytes)
  execution:check()
  return { value = value }
end

function BehaviorContext:store(policy, key, result, execution)
  result = BehaviorBoundary.copy(result, policy.maxBytes)
  execution.policy = policy
  execution.expires_at = BehaviorClock.now_ms() + policy.ttlMs
  BehaviorBoundary.adapter(self:adapter("caches", policy.name, "set"), { key, BehaviorBoundary.copy(result), execution, n = 3 }, execution)
end

function BehaviorContext:resilient(record, method, metadata, arguments, original, execution)
  local limit = metadata.concurrencyLimit
  local slot = record.name .. "." .. method
  if limit ~= nil then
    if (self.slots[slot] or 0) >= limit.limit then
      BehaviorError.raise("capacity", "Behavior method concurrency limit reached.")
    end
    self.slots[slot] = (self.slots[slot] or 0) + 1
  end
  local results = table.pack(pcall(self.retried, self, metadata, arguments, original, execution))
  if limit ~= nil then
    self.slots[slot] = self.slots[slot] - 1
  end
  if not results[1] then
    error(results[2], 0)
  end
  return results[2]
end

function BehaviorContext:retried(metadata, arguments, original, execution)
  local policy = metadata.retryable
  local attempt = 0
  while true do
    local results = table.pack(pcall(self.transacted, self, metadata, arguments, original, execution))
    if results[1] then
      return results[2]
    end
    local failure = results[2]
    if policy == nil or attempt >= policy.maxRetries or not BehaviorBoundary.retryable(failure) then
      error(failure, 0)
    end
    local wait = BehaviorBoundary.backoff(policy, attempt)
    attempt = attempt + 1
    execution:check()
    if wait > 0 and Behaviors.environment == "yon" then
      BehaviorClock.wait(wait)
    end
    execution:check()
  end
end

function BehaviorContext:transacted(metadata, arguments, original, execution)
  local policy = metadata.transactional
  if policy == nil or BehaviorOpenTransactions.of(policy.adapter) ~= nil then
    return self:body(metadata, arguments, original, execution)
  end
  local action = self:adapter("entities", policy.adapter, "execute")
  local control = function(step, identifier)
    local command = { operation = "transaction", action = step, transaction = identifier }
    return BehaviorBoundary.adapter(action, { command, execution, n = 2 }, execution)
  end
  local begun = BehaviorBoundary.copy(control("begin"), 4096)
  local identifier = type(begun) == "table" and begun.transaction or nil
  if type(identifier) ~= "string" or #identifier == 0 or #identifier > 128 then
    BehaviorBoundary.invalid("A transactional adapter must answer a bounded transaction identifier.")
  end
  local ok, result = pcall(BehaviorOpenTransactions.within, policy.adapter, identifier, function()
    return self:body(metadata, arguments, original, execution)
  end)
  if not ok then
    pcall(control, "rollback", identifier)
    error(result, 0)
  end
  control("commit", identifier)
  return result
end

function BehaviorContext:body(metadata, arguments, original, execution)
  if metadata.mapping ~= nil then
    return BehaviorBoundary.map_value(metadata.mapping, arguments)
  end
  local policy = metadata.httpExchange
  if policy ~= nil and policy.generated then
    local generated = {}
    for key, value in pairs(policy) do
      generated[key] = value
    end
    generated.path = BehaviorExchange.path(policy, arguments)
    return self:exchange(generated, BehaviorExchange.request(policy, arguments), execution)
  end
  local result = original(table.unpack(arguments, 1, arguments.n or #arguments))
  execution:check()
  if policy ~= nil then
    result = self:exchange(policy, result, execution)
  end
  return result
end

function BehaviorContext:exchange(policy, descriptor, execution)
  descriptor = BehaviorBoundary.copy(BehaviorBoundary.keys(descriptor, { "query", "headers", "body" }))
  local query = descriptor.query or {}
  local headers = descriptor.headers or {}
  if type(query) ~= "table" or type(headers) ~= "table" then
    BehaviorBoundary.invalid("Invalid HTTP descriptor.")
  end
  for _, value in pairs(query) do
    if not BehaviorExchange.scalar(value) then
      BehaviorBoundary.invalid("Invalid HTTP query.")
    end
  end
  for name, value in pairs(headers) do
    BehaviorContext.check_header(name, value)
  end
  if BehaviorContext.READS[policy.method] and descriptor.body ~= nil then
    BehaviorBoundary.invalid("HTTP reads cannot contain a request body.")
  end
  local request = {
    method = policy.method,
    path = policy.path,
    query = setmetatable(query, YonJson.OBJECT),
    headers = setmetatable(headers, YonJson.OBJECT),
    body = descriptor.body,
    redirect = "error",
    credentials = "omit",
    maxResponseBytes = BehaviorContext.MAX_RESPONSE_BYTES,
    signal = execution.signal,
    deadline = execution.deadline,
  }
  local action = self:adapter("http", policy.client, "exchange")
  return BehaviorBoundary.copy(BehaviorBoundary.adapter(action, { request, execution, n = 2 }, execution))
end

function BehaviorContext.check_header(name, value)
  local valid = type(name) == "string" and #name >= 1 and #name <= 128 and name:match(BehaviorContext.HEADER_NAME) ~= nil
    and type(value) == "string" and value:find("[\r\n\0]") == nil
  if not valid then
    BehaviorBoundary.invalid("Invalid HTTP header.")
  end
  if BehaviorContext.TRANSPORT_HEADERS[string.lower(name)] then
    BehaviorBoundary.invalid("HTTP header is transport-owned.")
  end
end

function BehaviorContext:mutate(record, metadata, arguments, original, execution)
  local put = metadata.cachePut ~= nil
  local mutation = metadata.cachePut or metadata.cacheEvict
  local mapped = { n = #mutation.arguments }
  for index, position in ipairs(mutation.arguments) do
    if position >= (arguments.n or #arguments) then
      BehaviorBoundary.invalid("Cache mutation argument is missing.")
    end
    mapped[index] = arguments[position + 1]
  end
  local target = record.methods[mutation.method]
  mapped = self:checked_arguments(mapped, { pageable = target.pageable })
  local policy = record.cache
  local action = self:adapter("caches", policy.name, put and "set" or "delete")
  local key = self:cache_key(record, mutation.method, mapped, execution, put and "cache:put" or "cache:evict")
  execution:check()
  local result = self:resilient(record, mutation.method, metadata, arguments, original, execution)
  result = BehaviorBoundary.copy(result, put and policy.maxBytes or 1048576)
  execution:check()
  if put then
    execution.policy = policy
    execution.expires_at = BehaviorClock.now_ms() + policy.ttlMs
    BehaviorBoundary.adapter(action, { key, BehaviorBoundary.copy(result), execution, n = 3 }, execution)
  else
    BehaviorBoundary.adapter(action, { key, execution, n = 2 }, execution)
  end
  self:revalidate_later(key)
  execution:check()
  return result
end

function BehaviorContext:revalidate_later(key)
  if #self.revalidations >= BehaviorContext.MAX_REVALIDATIONS then
    table.remove(self.revalidations, 1)
  end
  self.revalidations[#self.revalidations + 1] = key
end

function BehaviorContext:take_revalidation(key)
  for index, marked in ipairs(self.revalidations) do
    if marked == key then
      table.remove(self.revalidations, index)
      return true
    end
  end
  return false
end

function BehaviorContext:query(policy, arguments, execution)
  local where = {}
  for index, predicate in ipairs(policy.where) do
    where[index] = BehaviorContext.query_predicate(predicate, arguments)
  end
  local query = { kind = policy.operation, where = BehaviorBoundary.copy(where, 65536), orderBy = policy.orderBy }
  if policy.operation == "find" then
    query.page = BehaviorContext.query_page(policy, arguments)
  end
  local entity = policy.entity
  local command = { operation = "query", entity = entity.name, table = entity.table, idField = entity.id, query = query }
  command.transaction = BehaviorOpenTransactions.of(entity.adapter)
  local authorize = self.adapters.authorizeEntity
  local built_in = authorize == nil and self:builtin("entities", entity.adapter)
  if not built_in and not BehaviorBoundary.callable(authorize) then
    BehaviorBoundary.invalid("Store query requires entity authorization.")
  end
  local action = self:adapter("entities", entity.adapter, "execute")
  if not built_in and BehaviorBoundary.adapter(authorize, { BehaviorBoundary.copy(command), execution, n = 2 }, execution) ~= true then
    BehaviorError.raise("unauthorized", "Store query was not authorized.")
  end
  local result = BehaviorBoundary.copy(BehaviorBoundary.adapter(action, { BehaviorBoundary.copy(command), execution, n = 2 }, execution))
  BehaviorContext.check_query_result(policy.operation, result, query)
  execution:check()
  return result
end

function BehaviorContext.query_predicate(predicate, arguments)
  local entry = { field = predicate.field, operator = predicate.operator }
  if predicate.argument == nil then
    return entry
  end
  if predicate.argument >= (arguments.n or #arguments) then
    BehaviorBoundary.invalid("Query argument is missing.")
  end
  local value = arguments[predicate.argument + 1]
  if not BehaviorContext.bindable(predicate.operator, value) then
    BehaviorBoundary.invalid("Query requires finite scalar bindings or a bounded In array.")
  end
  entry.value = value
  return entry
end

function BehaviorContext.bindable(operator, value)
  if operator ~= "in" then
    return BehaviorContext.scalar(value)
  end
  if type(value) ~= "table" or #value > 100 then
    return false
  end
  for _, item in ipairs(value) do
    if not BehaviorContext.scalar(item) then
      return false
    end
  end
  return true
end

function BehaviorContext.scalar(value)
  local kind = type(value)
  return value == nil or kind == "string" or kind == "boolean" or kind == "number"
end

function BehaviorContext.query_page(policy, arguments)
  local index = policy.pageArgument
  if index ~= nil and index >= (arguments.n or #arguments) then
    BehaviorBoundary.invalid("Query page argument is missing.")
  end
  local rules = { defaultSize = policy.defaultSize, maxSize = policy.maxSize, sort = policy.fields }
  local given = index == nil and {} or arguments[index + 1]
  local page = BehaviorBoundary.page(given, rules)
  if #page.sort > 0 then
    BehaviorBoundary.invalid("Query ordering is annotation-owned.")
  end
  return page
end

function BehaviorContext.check_query_result(operation, result, query)
  local kind = BehaviorContext.RESULT_KINDS[operation]
  if kind == "list" then
    BehaviorContext.check_rows(result, query.page.size)
  elseif kind == "count" and not (math.type(result) == "integer" and result >= 0) then
    BehaviorBoundary.invalid("Query count result requires a nonnegative safe integer.")
  elseif kind == "boolean" and type(result) ~= "boolean" then
    BehaviorBoundary.invalid("Query exists result requires a boolean.")
  end
end

function BehaviorContext.check_rows(result, size)
  local rows = type(result) == "table" and #result <= size
  for _, row in ipairs(rows and result or {}) do
    rows = rows and type(row) == "table"
  end
  if not rows then
    BehaviorBoundary.invalid("Query find result requires a bounded array of objects.")
  end
end

function BehaviorContext:entity_value(record, operation, value)
  if operation == "find" then
    if type(value) ~= "table" then
      BehaviorBoundary.invalid("Entity pagination requires a map.")
    end
    return BehaviorBoundary.page(value, { defaultSize = 20, maxSize = 100, sort = { record.entity.id } })
  end
  if operation == "save" then
    value = BehaviorBoundary.copy(value)
    if type(value) ~= "table" then
      BehaviorBoundary.invalid("Entity value requires an object.")
    end
    BehaviorBoundary.entity_id(value[record.entity.id])
    return value
  end
  BehaviorBoundary.entity_id(value)
  return value
end

BehaviorHostCache = {}
BehaviorHostCache.__index = BehaviorHostCache

function BehaviorHostCache.new(name)
  return setmetatable({ name = name }, BehaviorHostCache)
end

function BehaviorHostCache:get(key)
  return BehaviorStorage.ask({ op = "cache.get", cache = self.name, key = key })
end

function BehaviorHostCache:set(key, value, execution)
  return BehaviorStorage.ask({ op = "cache.set", cache = self.name, key = key, value = value,
    expiresAt = execution.expires_at, maxEntries = execution.policy.maxEntries })
end

function BehaviorHostCache:delete(key)
  return BehaviorStorage.ask({ op = "cache.delete", cache = self.name, key = key })
end

BehaviorHostEntities = {}
BehaviorHostEntities.__index = BehaviorHostEntities

function BehaviorHostEntities:execute(command)
  return BehaviorStorage.ask({ op = "entity", command = command })
end

BehaviorBrowserHttp = {
  KEYS = { "method", "path", "query", "headers", "body", "redirect", "credentials", "maxResponseBytes" },
  CODES = {
    HTTP_REQUEST = true, HTTP_ORIGIN = true, HTTP_HEADER = true, HTTP_QUERY = true, HTTP_BODY = true,
    HTTP_LIMIT = true, HTTP_RESPONSE = true, HTTP_STATUS = true, HTTP_CAPACITY = true, HTTP_NETWORK = true,
    HTTP_MISSING = true, CANCELLED = true, DEADLINE = true,
  },
}
BehaviorBrowserHttp.__index = BehaviorBrowserHttp

function BehaviorBrowserHttp:exchange(request, execution)
  local browser = setmetatable({}, YonJson.OBJECT)
  for _, key in ipairs(BehaviorBrowserHttp.KEYS) do
    browser[key] = request[key]
  end
  browser.deadline = execution.deadline
  browser.publicRead = execution.public_read and request.method == "GET"
  browser.revalidate = execution.revalidate
  execution:check()
  local answer = YonJson.decode(TacTasks.exchange(YonJson.encode(browser)))
  local value = BehaviorBrowserHttp.value_of(answer)
  execution:check()
  return value
end

function BehaviorBrowserHttp.value_of(answer)
  if type(answer) ~= "table" then
    BehaviorBrowserHttp.fail("HTTP_RESPONSE")
  end
  if answer.error ~= nil then
    BehaviorBrowserHttp.fail(BehaviorBrowserHttp.code_of(answer.error))
  end
  if answer.value == nil then
    BehaviorBrowserHttp.fail("HTTP_RESPONSE")
  end
  if answer.value == YonNull then
    return nil
  end
  return answer.value
end

function BehaviorBrowserHttp.code_of(failure)
  local code = type(failure) == "table" and failure.code or nil
  if BehaviorBrowserHttp.CODES[code] then
    return code
  end
  return "HTTP_NETWORK"
end

function BehaviorBrowserHttp.fail(code)
  BehaviorError.raise(code, "The browser HTTP request failed (" .. code .. ").")
end

BehaviorBrowserSocket = {
  MAX_MESSAGE_BYTES = 65536,
  CODES = { HTTP_MISSING = true, HTTP_REQUEST = true, HTTP_CAPACITY = true, CANCELLED = true },
}

function BehaviorBrowserSocket.send(client, message)
  if TacTasks == nil or TacWebHost == nil or TachyonHost.test_unit then
    BehaviorBrowserSocket.fail("HTTP_MISSING")
  end
  local ok, text = pcall(YonJson.encode, message)
  if not ok or #text > BehaviorBrowserSocket.MAX_MESSAGE_BYTES then
    BehaviorBrowserSocket.fail("HTTP_BODY")
  end
  local answer = YonJson.decode(TacTasks.send_socket(client, text))
  if type(answer) ~= "table" or answer.error ~= nil then
    BehaviorBrowserSocket.fail(BehaviorBrowserSocket.code_of(type(answer) == "table" and answer.error or nil))
  end
  return nil
end

function BehaviorBrowserSocket.code_of(failure)
  local code = type(failure) == "table" and failure.code or nil
  if BehaviorBrowserSocket.CODES[code] then
    return code
  end
  return "HTTP_NETWORK"
end

function BehaviorBrowserSocket.fail(code)
  BehaviorError.raise(code, "The socket message was not sent (" .. code .. ").")
end

BehaviorStorage = {}

function BehaviorStorage.adapter(group, name)
  if Behaviors.environment ~= "tac" or TacWebHost == nil then
    return nil
  end
  if group == "caches" then
    return BehaviorHostCache.new(name)
  end
  if group == "entities" and name == "indexeddb" then
    return setmetatable({}, BehaviorHostEntities)
  end
  if group == "http" and TacTasks ~= nil then
    return setmetatable({}, BehaviorBrowserHttp)
  end
  return nil
end

function BehaviorStorage.ask(request)
  local answer = TacWebHost.ask(request)
  if type(answer.error) == "string" then
    BehaviorError.raise("storage", answer.error)
  end
  if type(answer.error) == "table" then
    BehaviorError.raise("storage", "Tachyon storage did not answer.")
  end
  if answer.value == YonNull then
    return nil
  end
  return answer.value
end

BehaviorRepository = {}
BehaviorRepository.__index = BehaviorRepository

function BehaviorRepository.new(context, record)
  return setmetatable({ context = context, record = record }, BehaviorRepository)
end

function BehaviorRepository:find(page)
  local rules = { defaultSize = 20, maxSize = 100, sort = { self.record.entity.id } }
  return self.context:entity_operation(self.record, "find", BehaviorBoundary.page(page or {}, rules))
end

function BehaviorRepository:save(value)
  return self.context:entity_operation(self.record, "save", value)
end

function BehaviorRepository:delete(id)
  return self.context:entity_operation(self.record, "delete", id)
end

BehaviorValidation = {
  BLANKS = { "\u{0085}", "\u{00A0}", "\u{1680}", "\u{2000}", "\u{2001}", "\u{2002}", "\u{2003}", "\u{2004}", "\u{2005}",
    "\u{2006}", "\u{2007}", "\u{2008}", "\u{2009}", "\u{200A}", "\u{2028}", "\u{2029}", "\u{202F}", "\u{205F}", "\u{3000}" },
  RULES = { notNull = "Required", notBlank = "NotEmpty", size = "Length", min = "Min", max = "Max" },
}

function BehaviorValidation.error(path)
  BehaviorError.raise("validation", "Behavior validation failed.", path)
end

function BehaviorValidation.reject(violations, path)
  local failure = { behavior_code = "validation", message = "Behavior validation failed.", path = path, violations = violations }
  error(setmetatable(failure, BehaviorError), 0)
end

function BehaviorValidation.apply(schema, input)
  local value = BehaviorValidation.object(input)
  local _, paths = BehaviorValidation.violations(BehaviorValidation.fields(schema), value)
  if paths[1] ~= nil then
    BehaviorValidation.error(paths[1])
  end
  return value
end

function BehaviorValidation.guard(schema, input)
  local fields = BehaviorValidation.fields(schema)
  local ok, value = pcall(BehaviorValidation.object, input)
  if not ok then
    BehaviorValidation.reject({ { field = "", rule = "Checked" } }, value.path)
  end
  local violations, paths = BehaviorValidation.violations(fields, value)
  if paths[1] ~= nil then
    BehaviorValidation.reject(violations, paths[1])
  end
  return value
end

function BehaviorValidation.object(input)
  local ok, value = pcall(BehaviorBoundary.copy, input)
  if not ok then
    BehaviorValidation.error("$.json")
  end
  local listed = type(value) == "table" and next(value) ~= nil and YonJson.is_list(value)
  if type(value) ~= "table" or listed then
    BehaviorValidation.error("$.object")
  end
  return value
end

function BehaviorValidation.fields(schema)
  if type(schema) ~= "table" or schema.fields == nil then
    BehaviorValidation.error("$.schema")
  end
  return schema.fields
end

function BehaviorValidation.violations(fields, value)
  local names = {}
  for name in pairs(fields) do
    names[#names + 1] = name
  end
  table.sort(names)
  local violations, paths = {}, {}
  for _, field in ipairs(names) do
    for _, key in ipairs(BehaviorValidation.broken(fields[field], value[field])) do
      violations[#violations + 1] = { field = field, rule = BehaviorValidation.RULES[key] }
      paths[#paths + 1] = field .. "." .. key
    end
  end
  return violations, paths
end

function BehaviorValidation.blank(text)
  for _, space in ipairs(BehaviorValidation.BLANKS) do
    text = text:gsub(space, "")
  end
  return text:find("%S") == nil
end

function BehaviorValidation.broken(rules, data)
  local keys = {}
  if data == YonNull then
    data = nil
  end
  if rules.notNull and data == nil then
    keys[#keys + 1] = "notNull"
  end
  if rules.notBlank and (type(data) ~= "string" or BehaviorValidation.blank(data)) then
    keys[#keys + 1] = "notBlank"
  end
  if data == nil then
    return keys
  end
  if rules.size ~= nil and not BehaviorValidation.size_fits(rules.size, data) then
    keys[#keys + 1] = "size"
  end
  local number = type(data) == "number"
  if rules.min ~= nil and (not number or data < rules.min) then
    keys[#keys + 1] = "min"
  end
  if rules.max ~= nil and (not number or data > rules.max) then
    keys[#keys + 1] = "max"
  end
  return keys
end

function BehaviorValidation.size_fits(limits, data)
  local size = -1
  if type(data) == "string" then
    size = #data
  elseif type(data) == "table" then
    size = 0
    for _ in pairs(data) do
      size = size + 1
    end
  end
  return size >= limits.min and size <= limits.max
end

BehaviorProperties = {
  WHOLE = "^%-?%d+$",
  NUMBER = "^%-?%d+%.?%d*[eE]?[%+%-]?%d*$",
  KEY = "^[A-Z][A-Z0-9_]*$",
  SEGMENT = "^[a-z][a-z0-9]*$",
  PROFILE = "^[%a_$][%w_$]*$",
}

function BehaviorProperties.error(code)
  BehaviorError.raise(code, "Invalid or inactive behavior configuration.")
end

function BehaviorProperties.source(input, code)
  local ok, values = pcall(BehaviorProperties.flat_source, input, code)
  if ok then
    return values
  end
  if BehaviorError.is(values) and values.behavior_code == code then
    error(values, 0)
  end
  BehaviorProperties.error(code)
end

function BehaviorProperties.flat_source(input, code)
  local values = BehaviorBoundary.copy(input, 65536)
  if type(values) == "table" and (values.tac ~= nil or values.yon ~= nil) then
    values = BehaviorProperties.environment_values(values)
  end
  if type(values) ~= "table" then
    BehaviorProperties.error(code)
  end
  local prefix = Behaviors.environment == "tac" and "TAC_" or "YON_"
  local count = 0
  for key, value in pairs(values) do
    count = count + 1
    local valid = type(key) == "string" and #key <= 256 and key:sub(1, #prefix) == prefix and key:match(BehaviorProperties.KEY) ~= nil
      and type(value) == "string" and #value <= 4096
    if not valid or count > 256 then
      BehaviorProperties.error(code)
    end
  end
  return values
end

function BehaviorProperties.environment_values(values)
  local prefix = Behaviors.environment .. "."
  local normalized = {}
  for path, value in pairs(BehaviorProperties.application(values)) do
    if path:sub(1, #prefix) == prefix then
      normalized[path:upper():gsub("%.", "_")] = value
    end
  end
  return normalized
end

function BehaviorProperties.select(record, options)
  local profiles = options.profiles or {}
  BehaviorProperties.check_profiles(profiles)
  local values = BehaviorProperties.source(options.properties or {}, "configuration")
  if record.profiles ~= nil and not BehaviorProperties.overlaps(profiles, record.profiles) then
    BehaviorProperties.error("inactive-configuration")
  end
  if record.conditionalOnProperty ~= nil then
    BehaviorProperties.check_condition(record.conditionalOnProperty, values)
  end
  local selected = {}
  for key, value in pairs(options) do
    selected[key] = value
  end
  selected.profiles = profiles
  selected.properties = values
  return selected
end

function BehaviorProperties.check_profiles(profiles)
  if type(profiles) ~= "table" or #profiles > 32 then
    BehaviorProperties.error("configuration")
  end
  local seen = {}
  for _, name in ipairs(profiles) do
    if type(name) ~= "string" or #name > 64 or name:match(BehaviorProperties.PROFILE) == nil or seen[name] then
      BehaviorProperties.error("configuration")
    end
    seen[name] = true
  end
end

function BehaviorProperties.overlaps(first, second)
  for _, name in ipairs(first) do
    for _, other in ipairs(second) do
      if name == other then
        return true
      end
    end
  end
  return false
end

function BehaviorProperties.check_condition(condition, values)
  for key in pairs(values) do
    if key:sub(1, #condition.name + 1) == condition.name .. "_" then
      BehaviorProperties.error("inactive-configuration")
    end
  end
  local matches = condition.matchIfMissing
  if values[condition.name] ~= nil then
    matches = values[condition.name] == condition.havingValue
  end
  if not matches then
    BehaviorProperties.error("inactive-configuration")
  end
end

function BehaviorProperties.bind(policy, input)
  if policy == nil then
    BehaviorProperties.error("configuration-properties")
  end
  local values = BehaviorProperties.source(input, "configuration-properties")
  local names = {}
  for _, descriptor in pairs(policy.fields) do
    names[policy.prefix .. descriptor.name] = true
  end
  for name in pairs(values) do
    if name:sub(1, #policy.prefix) == policy.prefix and not names[name] then
      BehaviorProperties.error("configuration-properties")
    end
  end
  local result = setmetatable({}, YonJson.OBJECT)
  for field, descriptor in pairs(policy.fields) do
    result[field] = BehaviorProperties.value(descriptor, values, policy.prefix .. descriptor.name)
  end
  return BehaviorBoundary.copy(result, 65536)
end

function BehaviorProperties.application(input)
  local ok, flat = pcall(BehaviorProperties.flat_application, input)
  if not ok then
    BehaviorProperties.error("configuration-properties")
  end
  return flat
end

function BehaviorProperties.flat_application(input)
  local tree = BehaviorBoundary.copy(input, 65536)
  local flat = {}
  local found = false
  for _, root in ipairs({ "tac", "yon" }) do
    if type(tree) == "table" and tree[root] ~= nil then
      found = true
      if type(tree[root]) ~= "table" then
        BehaviorProperties.error("configuration-properties")
      end
      BehaviorProperties.flatten(tree[root], root, 0, flat)
    end
  end
  if not found then
    BehaviorProperties.error("configuration-properties")
  end
  return flat
end

function BehaviorProperties.flatten(value, path, depth, flat)
  if depth > 16 or #path > 256 then
    BehaviorProperties.error("configuration-properties")
  end
  if path == "yon.server.port" and math.type(value) ~= "integer" then
    BehaviorProperties.error("configuration-properties")
  end
  if type(value) == "table" and (next(value) == nil or not YonJson.is_list(value)) then
    BehaviorProperties.flatten_map(value, path, depth, flat)
  elseif type(value) == "table" or value == nil then
    flat[path .. "."] = ""
  else
    BehaviorProperties.flatten_scalar(value, path, flat)
  end
end

function BehaviorProperties.flatten_map(value, path, depth, flat)
  if depth > 0 and next(value) == nil then
    flat[path .. "."] = ""
    return
  end
  for key, child in pairs(value) do
    if type(key) ~= "string" or key:match(BehaviorProperties.SEGMENT) == nil then
      BehaviorProperties.error("configuration-properties")
    end
    BehaviorProperties.flatten(child, path .. "." .. key, depth + 1, flat)
  end
end

function BehaviorProperties.flatten_scalar(value, path, flat)
  local text = tostring(value)
  if math.type(value) == "float" and value == math.floor(value) then
    text = string.format("%.1f", value)
  end
  if #text > 4096 then
    BehaviorProperties.error("configuration-properties")
  end
  flat[path] = text
end

function BehaviorProperties.values(policy, input)
  if policy == nil then
    BehaviorProperties.error("configuration-properties")
  end
  local flat = BehaviorProperties.application(input)
  local result = setmetatable({}, YonJson.OBJECT)
  for field, descriptor in pairs(policy) do
    result[field] = BehaviorProperties.value(descriptor, flat, descriptor.path)
  end
  return BehaviorBoundary.copy(result, 65536)
end

function BehaviorProperties.value(descriptor, values, name)
  for path in pairs(values) do
    if path:sub(1, #name + 1) == name .. "." then
      BehaviorProperties.error("configuration-properties")
    end
  end
  local text = values[name]
  if text == nil then
    if descriptor.default == nil then
      BehaviorProperties.error("configuration-properties")
    end
    return BehaviorProperties.typed_default(descriptor)
  end
  return BehaviorProperties.typed(descriptor, text)
end

function BehaviorProperties.typed_default(descriptor)
  local value = descriptor.default
  if descriptor.type == "integer" or descriptor.type == "number" then
    return BehaviorProperties.number(descriptor, value)
  end
  return value
end

function BehaviorProperties.typed(descriptor, text)
  local kind = descriptor.type
  if kind == "string" then
    return text
  end
  if kind == "boolean" then
    if text ~= "true" and text ~= "false" then
      BehaviorProperties.error("configuration-properties")
    end
    return text == "true"
  end
  local pattern = kind == "integer" and BehaviorProperties.WHOLE or BehaviorProperties.NUMBER
  if text:match(pattern) == nil or tonumber(text) == nil then
    BehaviorProperties.error("configuration-properties")
  end
  return BehaviorProperties.number(descriptor, tonumber(text))
end

function BehaviorProperties.number(descriptor, value)
  local valid = type(value) == "number" and value == value and math.abs(value) <= BehaviorBoundary.MAX_SAFE_INTEGER
    and (descriptor.type ~= "integer" or value == math.floor(value))
    and (descriptor.min == nil or value >= descriptor.min) and (descriptor.max == nil or value <= descriptor.max)
  if not valid then
    BehaviorProperties.error("configuration-properties")
  end
  if descriptor.type == "integer" then
    return math.tointeger(value)
  end
  return value
end

Behaviors = { records = {}, environment = "yon" }

function Behaviors.register(records_json, environment, module_id)
  Behaviors.environment = environment
  for name, record in pairs(YonJson.decode(records_json)) do
    record.module = module_id
    record.methods = record.methods or {}
    Behaviors.records[name] = record
  end
end

function Behaviors.create(setup, options)
  local name = BehaviorBoundary.short_name(setup)
  local record = Behaviors.records[name]
  if record == nil or not record.configuration then
    BehaviorBoundary.invalid("Setup must be source-local.")
  end
  options = BehaviorProperties.select(record, options or {})
  local context = BehaviorContext.new(Behaviors.records, options)
  local ok, failure = pcall(context.configure, context, function(execution)
    return setup.configure(execution)
  end)
  if not ok then
    context:dispose()
    error(failure, 0)
  end
  return context
end

function Behaviors.construct(type_table, values)
  local instance = {}
  for field, value in pairs(values or {}) do
    if type(field) ~= "string" or field:match("^[%l_][%w_]*$") == nil then
      BehaviorBoundary.invalid("Constructor fields must be plain field names.")
    end
    instance[field] = value
  end
  return setmetatable(instance, { __index = type_table })
end

function Behaviors.properties(type_table, values)
  return BehaviorProperties.bind((Behaviors.records[BehaviorBoundary.short_name(type_table)] or {}).configurationProperties, values)
end

function Behaviors.values(type_table, application)
  return BehaviorProperties.values((Behaviors.records[BehaviorBoundary.short_name(type_table)] or {}).values, application)
end

function Behaviors.validate(type_table, value)
  return BehaviorValidation.apply((Behaviors.records[BehaviorBoundary.short_name(type_table)] or {}).validated, value)
end
