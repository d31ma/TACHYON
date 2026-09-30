RuntimeContract = {}
RuntimeContract.__index = RuntimeContract

function RuntimeContract.new()
  Behaviors.register("{}", "yon", string.rep("a", 64))
  return setmetatable({
    failures = {},
    state = { hit = { hit = false }, authorization = 0, calls = 0, saved = 0 },
    attempts = 0,
    log = {},
    sequence = 0,
  }, RuntimeContract)
end

function RuntimeContract:fail(message)
  self.failures[#self.failures + 1] = message
end

function RuntimeContract:reject(label, code, action)
  local ok, failure = pcall(action)
  if ok then
    self:fail(label .. ": unexpectedly accepted")
  elseif not BehaviorError.is(failure) then
    self:fail(label .. ": leaked " .. tostring(failure))
  elseif failure.behavior_code ~= code then
    self:fail(label .. ": wrong code " .. failure.behavior_code)
  end
end

function RuntimeContract:expect(passed, message)
  if not passed then
    self:fail(message)
  end
end

function RuntimeContract:service_context(signal, entity)
  local record = {
    name = "Service",
    module = string.rep("a", 64),
    cache = { name = "data", ttlMs = 1000, maxBytes = 1024, maxEntries = 10 },
    methods = { read = { cacheable = { keyArgs = { 0 } } } },
  }
  local context = BehaviorContext.new({ Service = record, Product = entity }, { partition = "tenant", signal = signal })
  context:configure(function()
    return self:service_adapters()
  end)
  return context
end

function RuntimeContract:service_adapters()
  local state = self.state
  return {
    authorize = function()
      state.authorization = state.authorization + 1
      return true
    end,
    caches = { data = {
      get = function()
        return state.hit
      end,
      set = function(_, _, value, execution)
        state.hit = { hit = true, value = value, expiresAt = execution.expires_at }
      end,
      delete = function()
        state.hit = { hit = false }
      end,
    } },
    entities = { store = {
      execute = function(_, command)
        state.saved = state.saved + 1
        return command.operation
      end,
    } },
  }
end

function RuntimeContract:read(context, service)
  return context:invoke(service, "Service", "read", table.pack({ id = 1 }), function(value)
    self.state.calls = self.state.calls + 1
    return value
  end)
end

function RuntimeContract:check_cache(context, service)
  self:expect(self:read(context, service).id == 1, "cache contract: first read")
  self:expect(self:read(context, service).id == 1, "cache contract: second read")
  self:expect(self.state.authorization == 2 and self.state.calls == 1, "cache/auth contract")
  context:evict(service, "read", table.pack({ id = 1 }))
  self:read(context, service)
  self:expect(self.state.calls == 2, "eviction contract")
  self.state.hit = { hit = true, value = YonNull, expiresAt = BehaviorClock.now_ms() + 100000 }
  self:reject("cache excessive TTL", "invalid", function()
    self:read(context, service)
  end)
end

function RuntimeContract:check_repository(context, entity)
  local repository = context:repository(Product)
  self:reject("save missing id", "invalid", function()
    repository:save({ name = "x" })
  end)
  self:reject("save unsafe id", "invalid", function()
    repository:save({ id = true })
  end)
  self:reject("unsupported entity operation", "invalid", function()
    context:entity_operation(entity, "drop-table", {})
  end)
  self:reject("direct entity find bypass", "invalid", function()
    context:entity_operation(entity, "find", { size = 10000 })
  end)
  self:expect(repository:save({ id = "one" }) == "save", "entity contract: save")
  self:expect(repository:delete("one") == "delete", "entity contract: delete")
end

function RuntimeContract:check_boundaries()
  self:reject("whitespace partition", "invalid", function()
    BehaviorContext.new({}, { partition = "  ", signal = BehaviorSignal.new() })
  end)
  local page_rules = { defaultSize = 20, maxSize = 100, sort = {} }
  self:reject("negative page", "invalid", function()
    BehaviorBoundary.page({ page = -1 }, page_rules)
  end)
  self:reject("nonfinite JSON", "invalid", function()
    BehaviorBoundary.copy(math.huge)
  end)
  self:reject("invalid UTF8 JSON", "invalid", function()
    BehaviorBoundary.copy("\xff")
  end)
  self:reject("large JSON", "invalid", function()
    BehaviorBoundary.copy(string.rep("x", 1048577))
  end)
  self:reject("unsafe field", "invalid", function()
    BehaviorBoundary.copy({ __proto__ = 1 })
  end)
  local optional_schema = { fields = { name = { size = { min = 1, max = 16 } } } }
  self:reject("a list is not a DTO object", "validation", function()
    BehaviorValidation.apply(optional_schema, { 1, 2 })
  end)
  self:expect(next(BehaviorValidation.apply(optional_schema, {})) == nil, "empty DTO object shape lost")
  self:reject("a blank name", "validation", function()
    BehaviorValidation.apply({ fields = { name = { notBlank = true } } }, { name = " \u{00A0}" })
  end)
end

function RuntimeContract:resilient_context()
  local resilient = { name = "Orders", methods = {
    flaky = { retryable = { maxRetries = 3, delayMs = 0, multiplier = 1, maxDelayMs = 100, jitterMs = 0 } },
    guarded = { concurrencyLimit = { limit = 1, policy = "reject" } },
  } }
  local context = BehaviorContext.new({ Orders = resilient }, { partition = "tenant", signal = BehaviorSignal.new() })
  context:configure(function()
    return {}
  end)
  return context
end

function RuntimeContract:check_mapping()
  local fields = {
    { target = "enabled", source = "enabled" },
    { target = "items", source = "items", fields = { { target = "name", source = "name" } } },
  }
  local value = BehaviorBoundary.map_value({ argument = 0, fields = fields }, {
    { enabled = false, items = {}, secret = "private" },
  })
  self:expect(value.enabled == false, "mapping loses false")
  self:expect(YonJson.encode(value.items) == "[]", "mapping loses an empty list")
  self:expect(value.secret == nil, "mapping leaks a secret")
  local object = BehaviorBoundary.map_value({ argument = 0, fields = {
    { target = "detail", source = "detail", fields = {} },
  } }, { { detail = setmetatable({}, YonJson.OBJECT) } })
  self:expect(YonJson.encode(object.detail) == "{}", "mapping loses an empty object")
end

function RuntimeContract:check_retries(context, orders)
  self.attempts = 0
  local flaky = context:invoke(orders, "Orders", "flaky", table.pack(), function()
    self.attempts = self.attempts + 1
    if self.attempts < 3 then
      error("transient")
    end
    return "ok"
  end)
  self:expect(flaky == "ok" and self.attempts == 3, "retry contract: " .. self.attempts .. " attempts")
  self.attempts = 0
  local ok = pcall(context.invoke, context, orders, "Orders", "flaky", table.pack(), function()
    self.attempts = self.attempts + 1
    error("permanent")
  end)
  self:expect(not ok, "exhausted retries: unexpectedly accepted")
  self:expect(self.attempts == 4, "retry bound: " .. self.attempts .. " attempts")
end

function RuntimeContract:check_concurrency_limit(context, orders)
  local nested = function()
    return context:invoke(orders, "Orders", "guarded", table.pack(), function()
      return "nested"
    end)
  end
  self:reject("re-entrant concurrency limit", "capacity", function()
    context:invoke(orders, "Orders", "guarded", table.pack(), nested)
  end)
  local free = context:invoke(orders, "Orders", "guarded", table.pack(), function()
    return "free"
  end)
  self:expect(free == "free", "limit contract: slot not released")
end

function RuntimeContract:check_resilience()
  local context = self:resilient_context()
  local orders = { behaviors = context }
  self:check_retries(context, orders)
  self:check_concurrency_limit(context, orders)
end

function RuntimeContract:execute_ledger(command)
  if command.operation ~= "transaction" then
    self.log[#self.log + 1] = command.operation .. "@" .. (command.transaction or "none")
    return { id = "one" }
  end
  local entry = command.action
  if command.transaction ~= nil then
    entry = entry .. ":" .. command.transaction
  end
  self.log[#self.log + 1] = entry
  if command.action ~= "begin" then
    return { ok = true }
  end
  self.sequence = self.sequence + 1
  return { transaction = "t" .. self.sequence }
end

function RuntimeContract:ledger_context()
  local transactional = { name = "Ledger", methods = { post = { transactional = { adapter = "ledger" } } } }
  local ledger_entity = { name = "Entry", entity = { adapter = "ledger", id = "id" }, table = "entries", methods = {} }
  local context = BehaviorContext.new({ Ledger = transactional, Entry = ledger_entity }, { partition = "tenant", signal = BehaviorSignal.new() })
  context:configure(function()
    return { entities = { ledger = { execute = function(_, command)
      return self:execute_ledger(command)
    end } } }
  end)
  return context
end

function RuntimeContract:post(context, ledger, fail)
  return context:invoke(ledger, "Ledger", "post", table.pack(), function()
    context:repository(Entry):save({ id = "one" })
    if fail then
      error("domain rejected the entry")
    end
    return "posted"
  end)
end

function RuntimeContract:check_transactions()
  self.log = {}
  self.sequence = 0
  local context = self:ledger_context()
  local ledger = { behaviors = context }
  self:expect(self:post(context, ledger, false) == "posted", "transaction contract")
  self:expect(table.concat(self.log, ",") == "begin,save@t1,commit:t1", "transaction contract: " .. table.concat(self.log, ","))
  self.log = {}
  local ok = pcall(self.post, self, context, ledger, true)
  self:expect(not ok, "a failing transactional body was accepted")
  self:expect(table.concat(self.log, ",") == "begin,save@t2,rollback:t2", "rollback contract: " .. table.concat(self.log, ","))
  self.log = {}
  context:repository(Entry):save({ id = "two" })
  self:expect(table.concat(self.log, ",") == "save@none", "transaction leaked past its method: " .. table.concat(self.log, ","))
end

function RuntimeContract:check_cancellation(signal, context, service)
  signal:abort()
  self:reject("owner abort", "cancelled", function()
    self:read(context, service)
  end)
  local owner = BehaviorSignal.new()
  local child = BehaviorSignal.new()
  local execution = BehaviorExecution.new("tenant", "yon", child, owner, 1000)
  owner:abort()
  self:expect(execution.signal:aborted(), "adapter cancellation view does not observe owner abort")
end

function RuntimeContract:run()
  Product = { __name = "Product" }
  Entry = { __name = "Entry" }
  local entity = { name = "Product", entity = { adapter = "store", id = "id" }, table = "products", methods = {} }
  local signal = BehaviorSignal.new()
  local context = self:service_context(signal, entity)
  local service = { behaviors = context }
  self:check_cache(context, service)
  self:check_repository(context, entity)
  self:check_boundaries()
  self:check_mapping()
  self:check_resilience()
  self:check_transactions()
  self:check_cancellation(signal, context, service)
  if #self.failures > 0 then
    error(table.concat(self.failures, "\n"), 0)
  end
  print("Lua behavior boundary contracts passed")
end

RuntimeContract.new():run()
