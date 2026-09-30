unless defined?(BehaviorContext)
class BehaviorError < StandardError
  attr_reader :behavior_code, :path

  def initialize(behavior_code, message, path = nil)
    super(message)
    @behavior_code = behavior_code
    @path = path
  end
end

class BehaviorSignal
  def initialize
    @cancelled = false
  end

  def abort
    @cancelled = true
  end

  def aborted?
    @cancelled
  end
end

class BehaviorClock
  def self.now_ms
    (Time.now.to_f * 1000).floor
  end

  def self.monotonic_ms
    return Process.clock_gettime(Process::CLOCK_MONOTONIC, :millisecond) if defined?(Process::CLOCK_MONOTONIC)
    now_ms
  end
end

class BehaviorCancellation
  def initialize(context, owner, until_ms)
    @context = context
    @owner = owner
    @until_ms = until_ms
  end

  def aborted?
    @context.aborted? || @owner.aborted? || BehaviorClock.monotonic_ms >= @until_ms
  end
end

class BehaviorExecution
  attr_reader :partition, :environment, :deadline, :signal
  attr_accessor :policy, :expires_at

  def initialize(partition, environment, signal, owner, timeout_ms)
    @partition = partition
    @environment = environment
    @context_signal = signal
    @owner = owner
    @deadline = BehaviorClock.now_ms + timeout_ms
    @until_ms = BehaviorClock.monotonic_ms + timeout_ms
    @signal = BehaviorCancellation.new(signal, owner, @until_ms)
    @policy = {}
    @expires_at = 0
  end

  def check
    raise BehaviorError.new("cancelled", "Behavior operation was cancelled.") if @context_signal.aborted? || @owner.aborted?
    raise BehaviorError.new("timeout", "Behavior operation exceeded its deadline.") if BehaviorClock.monotonic_ms >= @until_ms
  end
end

class BehaviorBoundary
  MAX_SAFE_INTEGER = 9_007_199_254_740_991
  UNSAFE_FIELDS = ["__proto__", "prototype", "constructor", "behaviors"].freeze
  ADAPTER_NAME = /\A[A-Za-z_$][A-Za-z0-9_$]{0,127}\z/
  GATEWAY_FAILURES = ["invalid", "timeout", "capacity", "adapter"].freeze

  def self.invalid(message)
    raise BehaviorError.new("invalid", message)
  end

  def self.short_name(type)
    name = type.is_a?(Module) ? type.name.to_s : type.to_s
    name.split("::").last.to_s
  end

  def self.named(value)
    return value unless value.is_a?(Hash)
    result = {}
    value.each { |key, item| result[key.is_a?(Symbol) ? key.to_s : key] = item }
    result
  end

  def self.gateway(key, arguments)
    invalid_gateway = BehaviorError.new("adapter", "Gateway call failed.")
    raise invalid_gateway unless defined?(YonGateway)
    answer = YonGateway.call(key, arguments)
    return answer["result"] if answer.is_a?(Hash) && answer.key?("result")
    kind = answer.is_a?(Hash) ? answer["error"] : "adapter"
    kind = "adapter" unless GATEWAY_FAILURES.include?(kind)
    raise BehaviorError.new(kind, "Gateway call failed.")
  end

  def self.keys(value, allowed)
    value = named(value)
    invalid("Unknown or invalid behavior options.") unless value.is_a?(Hash) && (value.keys - allowed).empty?
    value
  end

  def self.copy(value, limit = 1_048_576)
    counter = { "nodes" => 0, "characters" => 0 }
    result = walk(value, 0, counter, limit)
    invalid("Behavior JSON exceeds byte bounds.") if encode(result).bytesize > limit
    result
  end

  def self.encode(value)
    JSON.generate(value)
  rescue BehaviorError
    raise
  rescue StandardError
    invalid("Behavior JSON is not valid UTF-8 JSON.")
  end

  def self.walk(value, depth, counter, limit)
    counter["nodes"] += 1
    invalid("Behavior JSON exceeds structural bounds.") if counter["nodes"] > 10_000 || depth > 32
    return value if value.nil? || value == true || value == false
    return value if value.is_a?(Integer) && value.abs <= MAX_SAFE_INTEGER
    return value if value.is_a?(Float) && value.finite?
    return text(value.to_s, counter, limit) if value.is_a?(String) || value.is_a?(Symbol)
    value = value.to_h if value.is_a?(Struct) || (defined?(Data) && value.is_a?(Data))
    return walk_list(value, depth, counter, limit) if value.is_a?(Array)
    return walk_map(value, depth, counter, limit) if value.is_a?(Hash)
    invalid("Behavior values must be finite JSON data.")
  end

  def self.text(value, counter, limit)
    invalid("Behavior JSON is not valid UTF-8 JSON.") if value.respond_to?(:valid_encoding?) && !value.valid_encoding?
    counter["characters"] += value.bytesize
    invalid("Behavior JSON exceeds byte bounds.") if counter["characters"] > limit
    value
  end

  def self.walk_list(items, depth, counter, limit)
    invalid("Behavior collection exceeds bounds.") if items.size > 10_000
    items.map { |item| walk(item, depth + 1, counter, limit) }
  end

  def self.walk_map(items, depth, counter, limit)
    invalid("Behavior collection exceeds bounds.") if items.size > 10_000
    result = {}
    named(items).sort_by { |key, _| key.to_s }.each do |key, item|
      unsafe = !key.is_a?(String) || UNSAFE_FIELDS.include?(key) || (key == "tac" && depth != 0)
      invalid("Unsafe behavior JSON field.") if unsafe
      walk(key, depth + 1, counter, limit)
      result[key] = walk(item, depth + 1, counter, limit)
    end
    result
  end

  def self.page(value, policy)
    value = keys(value, ["page", "size", "offset", "sort"])
    copy(value, 4096)
    page = value.key?("page") ? value["page"] : 0
    size = value.key?("size") ? value["size"] : policy["defaultSize"]
    bounded = page.is_a?(Integer) && size.is_a?(Integer) && page >= 0 && page <= 1_000_000 &&
      size >= 1 && size <= policy["maxSize"] && page * size <= 1_000_000
    invalid("Invalid behavior pagination bounds.") unless bounded
    invalid("Invalid behavior pagination offset.") if value.key?("offset") && value["offset"] != page * size
    { "page" => page, "size" => size, "offset" => page * size, "sort" => sort(value["sort"] || [], policy["sort"]) }
  end

  def self.sort(items, allowed)
    invalid("Invalid behavior sort.") unless items.is_a?(Array) && items.size <= 16
    seen = []
    items.map do |item|
      invalid("Invalid behavior sort.") unless item.is_a?(String)
      field, direction, extra = item.split(":")
      direction ||= "asc"
      invalid("Invalid behavior sort.") if extra || !allowed.include?(field) || !["asc", "desc"].include?(direction) || seen.include?(field)
      seen << field
      "#{field}:#{direction}"
    end
  end

  def self.adapter(function, arguments, execution)
    execution.check
    begin
      result = function.call(*arguments)
    rescue BehaviorError
      raise
    rescue StandardError
      raise BehaviorError.new("adapter", "Behavior adapter failed.")
    end
    execution.check
    result
  end

  def self.map_value(policy, arguments)
    value = named(arguments[policy["argument"]])
    invalid("A behavior mapping reads one JSON object argument.") unless value.is_a?(Hash)
    output = {}
    policy["fields"].each do |field|
      unless field.key?("source")
        output[field["target"]] = field["constant"]
        next
      end
      current = value
      field["source"].split(".").each do |name|
        current = named(current)
        current = current.is_a?(Hash) && current.key?(name) ? current[name] : nil
      end
      output[field["target"]] = current
    end
    copy(output)
  end

  def self.exchange_path(policy, arguments)
    path = policy["path"].dup
    (policy["pathVariables"] || {}).each do |name, index|
      value = arguments[index]
      text = value.is_a?(Integer) || value.is_a?(Float) || value == true || value == false ? value.to_s : value
      safe = text.is_a?(String) && text.match?(/\A[A-Za-z0-9._~-]{1,256}\z/) && text != "." && text != ".."
      invalid("A path variable may use only letters, digits and - _ . ~") unless safe
      path = path.gsub("{#{name}}", text)
    end
    path
  end

  def self.exchange_request(policy, arguments)
    pick = lambda do |index|
      invalid("A request argument is missing.") if index >= arguments.size
      arguments[index]
    end
    descriptor = { "query" => {}, "headers" => {} }
    (policy["query"] || {}).each { |name, index| descriptor["query"][name] = pick.call(index) }
    (policy["headers"] || {}).each { |name, index| descriptor["headers"][name] = pick.call(index) }
    descriptor["body"] = pick.call(policy["body"]) if policy.key?("body")
    descriptor
  end

  def self.retryable?(error)
    !error.is_a?(BehaviorError) || error.behavior_code == "adapter"
  end

  def self.backoff(policy, attempt)
    delay = [policy["delayMs"] * (policy["multiplier"]**attempt), policy["maxDelayMs"]].min
    (delay + rand * policy["jitterMs"]).round
  end

  def self.safe_path?(path)
    return false unless path.is_a?(String) && path.start_with?("/") && !path.start_with?("//") && path.bytesize <= 2048
    return false if path.match?(/[\\?#%\x00-\x20]/)
    path.split("/").none? { |part| part == "." || part == ".." }
  end

  def self.entity_id(id)
    valid = (id.is_a?(String) && !id.empty? && id.bytesize <= 256) || (id.is_a?(Integer) && id.abs <= MAX_SAFE_INTEGER)
    invalid("Invalid entity identifier.") unless valid
    copy(id, 1024)
  end

  def self.callable?(value)
    value.respond_to?(:call)
  end
end

class BehaviorOpenTransactions
  KEY = :tachyon_behavior_transactions

  def self.current
    open = defined?(Thread) ? Thread.current[KEY] : @open
    open || {}
  end

  def self.replace(open)
    if defined?(Thread)
      Thread.current[KEY] = open
    else
      @open = open
    end
  end

  def self.of(adapter)
    current[adapter]
  end

  def self.within(adapter, identifier)
    outer = current
    replace(outer.merge(adapter => identifier))
    begin
      yield
    ensure
      replace(outer)
    end
  end
end

class BehaviorContext
  ADAPTER_GROUPS = ["caches", "http", "entities"].freeze
  ADAPTER_KEYS = ["caches", "http", "entities", "authorize", "authorizeEntity"].freeze
  RESERVED_NAMES = ["__proto__", "prototype", "constructor", "behaviors", "tac"].freeze
  QUERY_RESULTS = { "find" => "array", "findOne" => "array", "count" => "count", "exists" => "boolean", "delete" => "count" }.freeze

  attr_reader :partition, :adapters

  def initialize(records, options)
    options = BehaviorBoundary.keys(options, ["partition", "signal", "timeoutMs", "profiles", "properties"])
    partition = options["partition"]
    signal = options["signal"]
    timeout = options.key?("timeoutMs") ? options["timeoutMs"] : 10_000
    valid = partition.is_a?(String) && !partition.strip.empty? && partition.bytesize <= 256 &&
      signal.is_a?(BehaviorSignal) && timeout.is_a?(Integer) && timeout >= 1 && timeout <= 60_000
    BehaviorBoundary.invalid("Explicit bounded partition, BehaviorSignal and timeout are required.") unless valid
    @records = records
    @partition = partition
    @owner = signal
    @signal = BehaviorSignal.new
    @timeout_ms = timeout
    @adapters = {}
    @slots = {}
    @active = 0
  end

  def configure(factory = nil, &block)
    factory ||= block
    execution = start
    begin
      adapters = BehaviorBoundary.keys(BehaviorBoundary.adapter(factory, [execution], execution), ADAPTER_KEYS)
      ADAPTER_GROUPS.each do |group|
        collection = BehaviorBoundary.named(adapters[group] || {})
        BehaviorBoundary.invalid("Invalid adapter collection.") unless collection.is_a?(Hash) && collection.size <= 128
        collection.each_key do |name|
          safe = name.is_a?(String) && name.match?(BehaviorBoundary::ADAPTER_NAME) && !RESERVED_NAMES.include?(name)
          BehaviorBoundary.invalid("Invalid adapter name.") unless safe
        end
        adapters[group] = collection
      end
      BehaviorBoundary.invalid("Authorize must be callable.") if adapters.key?("authorize") && !BehaviorBoundary.callable?(adapters["authorize"])
      BehaviorBoundary.invalid("Entity authorize must be callable.") if adapters.key?("authorizeEntity") && !BehaviorBoundary.callable?(adapters["authorizeEntity"])
      @adapters = adapters
    ensure
      @active -= 1
    end
  end

  def share(setup_key, adapters)
    @setup_key = setup_key
    @adapters = adapters
  end

  def dispose
    @signal.abort
    return unless @setup_key
    setup_key = @setup_key
    @setup_key = nil
    Behaviors.release_setup(setup_key)
  end

  def invoke(instance, type, method, arguments, original)
    record = record_of(instance, type, method)
    metadata = record["methods"][method]
    execution = start
    begin
      arguments = checked_arguments(arguments, metadata)
      return query(metadata["query"], arguments, execution) if metadata.key?("query")
      return mutate(record, metadata, arguments, original, execution) if metadata.key?("cachePut") || metadata.key?("cacheEvict")
      key = nil
      policy = record["cache"]
      if metadata.key?("cacheable")
        key = cache_key(record, method, arguments, execution, "cache:read")
        hit = cached(policy, key, execution)
        return hit[:value] if hit
      end
      execution.check
      result = BehaviorBoundary.copy(resilient(record, method, metadata, arguments, original, execution))
      store(policy, key, result, execution) if key
      execution.check
      result
    ensure
      @active -= 1
    end
  end

  def evict(instance, method, arguments)
    record = record_of(instance, BehaviorBoundary.short_name(instance.class), method)
    BehaviorBoundary.invalid("Eviction requires a cacheable method.") unless record["methods"][method].key?("cacheable")
    execution = start
    begin
      arguments = checked_arguments(arguments, record["methods"][method])
      key = cache_key(record, method, arguments, execution, "cache:evict")
      BehaviorBoundary.adapter(adapter("caches", record["cache"]["name"], "delete"), [key, execution], execution)
      nil
    ensure
      @active -= 1
    end
  end

  def rest_client(name)
    BehaviorBoundary.invalid("RestClient requires a configured http adapter name.") unless name.is_a?(String) && name.match?(BehaviorBoundary::ADAPTER_NAME)
    BehaviorRestClient.new(self, name)
  end

  def rest_exchange(name, method, path, descriptor)
    execution = start
    begin
      BehaviorBoundary.copy(exchange({ "client" => name, "method" => method, "path" => path }, descriptor, execution))
    ensure
      @active -= 1
    end
  end

  def repository(entity)
    record = @records[BehaviorBoundary.short_name(entity)]
    BehaviorBoundary.invalid("Store requires source-local Entity/Table.") unless record && record.key?("entity")
    BehaviorRepository.new(self, record)
  end

  def entity_operation(record, operation, value)
    BehaviorBoundary.invalid("Entity belongs to another module.") unless record.is_a?(Hash) && @records[record["name"]].equal?(record)
    known = record.key?("entity") && record.key?("table") && ["find", "save", "delete"].include?(operation)
    BehaviorBoundary.invalid("Unknown entity operation.") unless known
    value = entity_value(record, operation, value)
    execution = start
    begin
      field = { "find" => "page", "save" => "value", "delete" => "id" }[operation]
      command = { "operation" => operation, "entity" => record["name"], "table" => record["table"], "idField" => record["entity"]["id"] }
      command[field] = BehaviorBoundary.copy(value)
      joined = BehaviorOpenTransactions.of(record["entity"]["adapter"])
      command["transaction"] = joined if joined
      function = adapter("entities", record["entity"]["adapter"], "execute")
      BehaviorBoundary.copy(BehaviorBoundary.adapter(function, [command, execution], execution))
    ensure
      @active -= 1
    end
  end

  private

  def start
    execution = BehaviorExecution.new(@partition, Behaviors.environment, @signal, @owner, @timeout_ms)
    execution.check
    BehaviorBoundary.invalid("Behavior concurrency exceeds bounds.") if @active >= 64
    @active += 1
    execution
  end

  def adapter(group, name, operation)
    found = (@adapters[group] || {})[name]
    found = BehaviorStorage.adapter(group, name) if found.nil?
    function = if found.is_a?(Hash)
      BehaviorBoundary.named(found)[operation]
    elsif !found.nil? && found.respond_to?(operation)
      found.method(operation)
    end
    BehaviorBoundary.invalid("Required behavior adapter is missing.") unless BehaviorBoundary.callable?(function)
    function
  end

  def builtin?(group, name)
    (@adapters[group] || {})[name].nil? && !BehaviorStorage.adapter(group, name).nil?
  end

  def checked_arguments(arguments, metadata)
    arguments = arguments.dup
    if metadata.key?("valid")
      policy = metadata["valid"]
      arguments[policy["argument"]] = BehaviorValidation.apply(policy["schema"], arguments[policy["argument"]])
    end
    BehaviorBoundary.invalid("Too many behavior arguments.") if arguments.size > 32
    if metadata.key?("pageable")
      policy = metadata["pageable"]
      index = policy["argument"]
      given = BehaviorBoundary.named(arguments[index])
      BehaviorBoundary.invalid("Pagination requires an argument map.") unless index < arguments.size && given.is_a?(Hash)
      arguments[index] = BehaviorBoundary.page(given, policy)
    end
    arguments
  end

  def record_of(instance, type, method)
    record = @records[type]
    owned = BehaviorBoundary.short_name(instance.class) == type && record && record["methods"].key?(method) &&
      instance.instance_variable_get(:@behaviors).equal?(self)
    BehaviorBoundary.invalid("Behavior instance requires its source-local context.") unless owned
    record
  end

  def cache_key(record, method, arguments, execution, operation)
    selected = record["methods"][method]["cacheable"]["keyArgs"].map do |index|
      BehaviorBoundary.invalid("Cache key argument is missing.") if index >= arguments.size
      arguments[index]
    end
    selected = BehaviorBoundary.copy(selected, 4096)
    call = { "operation" => operation, "cache" => record["cache"]["name"], "type" => record["name"], "method" => method, "keyArguments" => BehaviorBoundary.copy(selected) }
    authorize = @adapters["authorize"]
    unless authorize.nil? && builtin?("caches", record["cache"]["name"])
      allowed = BehaviorBoundary.callable?(authorize) && BehaviorBoundary.adapter(authorize, [call, execution], execution) == true
      raise BehaviorError.new("unauthorized", "Behavior cache access denied.") unless allowed
    end
    BehaviorBoundary.encode(BehaviorBoundary.copy([@partition, record["cache"]["name"], record["module"], record["name"], method, selected], 4096))
  end

  def cached(policy, key, execution)
    hit = BehaviorBoundary.adapter(adapter("caches", policy["name"], "get"), [key, execution], execution)
    hit = BehaviorBoundary.keys(hit, ["hit", "value", "expiresAt"])
    BehaviorBoundary.invalid("Invalid cache response.") unless hit["hit"] == true || hit["hit"] == false
    return nil unless hit["hit"]
    BehaviorBoundary.invalid("Invalid cache expiry/value.") unless hit["expiresAt"].is_a?(Integer) && hit.key?("value")
    now = BehaviorClock.now_ms
    BehaviorBoundary.invalid("Cache expiry exceeds its policy.") if hit["expiresAt"] > now + policy["ttlMs"]
    return nil unless hit["expiresAt"] > now
    value = BehaviorBoundary.copy(hit["value"], policy["maxBytes"])
    execution.check
    { value: value }
  end

  def store(policy, key, result, execution)
    result = BehaviorBoundary.copy(result, policy["maxBytes"])
    execution.policy = policy
    execution.expires_at = BehaviorClock.now_ms + policy["ttlMs"]
    BehaviorBoundary.adapter(adapter("caches", policy["name"], "set"), [key, BehaviorBoundary.copy(result), execution], execution)
  end

  def resilient(record, method, metadata, arguments, original, execution)
    limit = metadata["concurrencyLimit"]
    slot = "#{record['name']}.#{method}"
    if limit
      raise BehaviorError.new("capacity", "Behavior method concurrency limit reached.") if @slots.fetch(slot, 0) >= limit["limit"]
      @slots[slot] = @slots.fetch(slot, 0) + 1
    end
    begin
      retried(metadata, arguments, original, execution)
    ensure
      if limit
        @slots[slot] -= 1
        @slots.delete(slot) if @slots[slot] <= 0
      end
    end
  end

  def retried(metadata, arguments, original, execution)
    policy = metadata["retryable"]
    attempt = 0
    begin
      transacted(metadata, arguments, original, execution)
    rescue StandardError => error
      raise if policy.nil? || attempt >= policy["maxRetries"] || !BehaviorBoundary.retryable?(error)
      wait = BehaviorBoundary.backoff(policy, attempt)
      attempt += 1
      execution.check
      sleep(wait / 1000.0) if wait > 0 && Behaviors.environment == "yon"
      execution.check
      retry
    end
  end

  def transacted(metadata, arguments, original, execution)
    policy = metadata["transactional"]
    return body(metadata, arguments, original, execution) if policy.nil? || BehaviorOpenTransactions.of(policy["adapter"])
    function = adapter("entities", policy["adapter"], "execute")
    control = lambda do |action, identifier = nil|
      command = { "operation" => "transaction", "action" => action }
      command["transaction"] = identifier if identifier
      BehaviorBoundary.adapter(function, [command, execution], execution)
    end
    begun = BehaviorBoundary.copy(control.call("begin"), 4096)
    identifier = begun.is_a?(Hash) ? begun["transaction"] : nil
    valid = identifier.is_a?(String) && !identifier.empty? && identifier.bytesize <= 128
    BehaviorBoundary.invalid("A transactional adapter must answer a bounded transaction identifier.") unless valid
    begin
      result = BehaviorOpenTransactions.within(policy["adapter"], identifier) { body(metadata, arguments, original, execution) }
    rescue StandardError
      begin
        control.call("rollback", identifier)
      rescue StandardError
        nil
      end
      raise
    end
    control.call("commit", identifier)
    result
  end

  def body(metadata, arguments, original, execution)
    return BehaviorBoundary.map_value(metadata["mapping"], arguments) if metadata.key?("mapping")
    policy = metadata["httpExchange"]
    if policy && policy["generated"]
      generated = policy.merge("path" => BehaviorBoundary.exchange_path(policy, arguments))
      return exchange(generated, BehaviorBoundary.exchange_request(policy, arguments), execution)
    end
    result = original.call(*arguments)
    execution.check
    result = exchange(policy, result, execution) if policy
    result
  end

  def exchange(policy, descriptor, execution)
    descriptor = BehaviorBoundary.copy(BehaviorBoundary.keys(descriptor, ["query", "headers", "body"]))
    query = descriptor["query"] || {}
    headers = descriptor["headers"] || {}
    BehaviorBoundary.invalid("Invalid HTTP descriptor.") unless query.is_a?(Hash) && headers.is_a?(Hash)
    query.each_value do |value|
      scalar = value.is_a?(String) || value.is_a?(Integer) || value.is_a?(Float) || value == true || value == false
      BehaviorBoundary.invalid("Invalid HTTP query.") unless scalar
    end
    headers.each { |name, value| check_header(name, value) }
    BehaviorBoundary.invalid("HTTP reads cannot contain a request body.") if ["GET", "HEAD"].include?(policy["method"]) && descriptor.key?("body")
    request = descriptor.merge(
      "method" => policy["method"], "path" => policy["path"], "query" => query, "headers" => headers,
      "redirect" => "error", "credentials" => "omit", "maxResponseBytes" => 1_048_576,
      "signal" => execution.signal, "deadline" => execution.deadline
    )
    function = adapter("http", policy["client"], "exchange")
    BehaviorBoundary.copy(BehaviorBoundary.adapter(function, [request, execution], execution))
  end

  def check_header(name, value)
    valid = name.is_a?(String) && name.match?(/\A[!#$%&'*+.^_`|~0-9A-Za-z-]{1,128}\z/) &&
      value.is_a?(String) && !value.match?(/[\r\n\x00]/)
    BehaviorBoundary.invalid("Invalid HTTP header.") unless valid
    owned = ["host", "connection", "content-length", "transfer-encoding", "cookie"].include?(name.downcase)
    BehaviorBoundary.invalid("HTTP header is transport-owned.") if owned
  end

  def mutate(record, metadata, arguments, original, execution)
    put = metadata.key?("cachePut")
    mutation = metadata[put ? "cachePut" : "cacheEvict"]
    mapped = mutation["arguments"].map do |index|
      BehaviorBoundary.invalid("Cache mutation argument is missing.") if index >= arguments.size
      arguments[index]
    end
    target = record["methods"][mutation["method"]]
    mapped = checked_arguments(mapped, target.key?("pageable") ? { "pageable" => target["pageable"] } : {})
    policy = record["cache"]
    function = adapter("caches", policy["name"], put ? "set" : "delete")
    key = cache_key(record, mutation["method"], mapped, execution, put ? "cache:put" : "cache:evict")
    execution.check
    result = resilient(record, mutation["method"], metadata, arguments, original, execution)
    result = BehaviorBoundary.copy(result, put ? policy["maxBytes"] : 1_048_576)
    execution.check
    if put
      execution.policy = policy
      execution.expires_at = BehaviorClock.now_ms + policy["ttlMs"]
      BehaviorBoundary.adapter(function, [key, BehaviorBoundary.copy(result), execution], execution)
    else
      BehaviorBoundary.adapter(function, [key, execution], execution)
    end
    execution.check
    result
  end

  def query(policy, arguments, execution)
    where = policy["where"].map { |predicate| query_predicate(predicate, arguments) }
    single = policy["operation"] == "findOne"
    query = { "kind" => single ? "find" : policy["operation"], "where" => BehaviorBoundary.copy(where, 65_536), "orderBy" => policy["orderBy"] }
    query["page"] = query_page(policy, arguments) if policy["operation"] == "find"
    query["page"] = BehaviorBoundary.page({ "size" => 1 }, { "defaultSize" => 1, "maxSize" => 1, "sort" => policy["fields"] }) if single
    entity = policy["entity"]
    command = { "operation" => "query", "entity" => entity["name"], "table" => entity["table"], "idField" => entity["id"], "query" => query }
    joined = BehaviorOpenTransactions.of(entity["adapter"])
    command["transaction"] = joined if joined
    authorize = @adapters["authorizeEntity"]
    built_in = authorize.nil? && builtin?("entities", entity["adapter"])
    BehaviorBoundary.invalid("Store query requires entity authorization.") unless built_in || BehaviorBoundary.callable?(authorize)
    function = adapter("entities", entity["adapter"], "execute")
    allowed = built_in || BehaviorBoundary.adapter(authorize, [BehaviorBoundary.copy(command), execution], execution) == true
    raise BehaviorError.new("unauthorized", "Store query was not authorized.") unless allowed
    result = BehaviorBoundary.copy(BehaviorBoundary.adapter(function, [BehaviorBoundary.copy(command), execution], execution))
    check_query_result(policy["operation"], result, query)
    execution.check
    single ? result.first : result
  end

  def query_predicate(predicate, arguments)
    entry = { "field" => predicate["field"], "operator" => predicate["operator"] }
    return entry unless predicate.key?("argument")
    BehaviorBoundary.invalid("Query argument is missing.") if predicate["argument"] >= arguments.size
    value = arguments[predicate["argument"]]
    scalar = lambda { |item| item.nil? || item.is_a?(String) || item == true || item == false || item.is_a?(Integer) || item.is_a?(Float) }
    valid = if predicate["operator"] == "in"
      value.is_a?(Array) && value.size <= 100 && value.all? { |item| scalar.call(item) }
    else
      scalar.call(value)
    end
    BehaviorBoundary.invalid("Query requires finite scalar bindings or a bounded In array.") unless valid
    entry["value"] = value
    entry
  end

  def query_page(policy, arguments)
    index = policy["pageArgument"]
    BehaviorBoundary.invalid("Query page argument is missing.") if !index.nil? && index >= arguments.size
    rules = { "defaultSize" => policy["defaultSize"], "maxSize" => policy["maxSize"], "sort" => policy["fields"] }
    page = BehaviorBoundary.page(index.nil? ? {} : arguments[index], rules)
    BehaviorBoundary.invalid("Query ordering is annotation-owned.") unless page["sort"].empty?
    page
  end

  def check_query_result(operation, result, query)
    case operation
    when "find", "findOne"
      rows = result.is_a?(Array) && result.size <= query["page"]["size"] && result.all? { |row| row.is_a?(Hash) }
      BehaviorBoundary.invalid("Query find result requires a bounded array of objects.") unless rows
    when "count", "delete"
      count = result.is_a?(Integer) && result >= 0 && result <= BehaviorBoundary::MAX_SAFE_INTEGER
      BehaviorBoundary.invalid("Query count and delete results require a nonnegative safe integer.") unless count
    when "exists"
      BehaviorBoundary.invalid("Query exists result requires a boolean.") unless result == true || result == false
    end
  end

  def entity_value(record, operation, value)
    case operation
    when "find"
      value = BehaviorBoundary.named(value)
      BehaviorBoundary.invalid("Entity pagination requires a map.") unless value.is_a?(Hash)
      BehaviorBoundary.page(value, { "defaultSize" => 20, "maxSize" => 100, "sort" => [record["entity"]["id"]] })
    when "save"
      value = BehaviorBoundary.copy(value)
      BehaviorBoundary.invalid("Entity value requires an object.") unless value.is_a?(Hash)
      BehaviorBoundary.entity_id(value[record["entity"]["id"]])
      value
    else
      BehaviorBoundary.entity_id(value)
      value
    end
  end
end

class BehaviorRestClient
  def initialize(context, name)
    @context = context
    @name = name
  end

  ["get", "post", "put", "patch", "delete", "head", "options"].each do |verb|
    define_method(verb) { |path| BehaviorRestCall.new(@context, @name, verb.upcase, path) }
  end
end

class BehaviorRestCall
  def initialize(context, name, method, path)
    BehaviorBoundary.invalid("RestClient requires a bounded origin-relative path without query, fragment or backslash.") unless BehaviorBoundary.safe_path?(path)
    @context = context
    @name = name
    @method = method
    @path = path
    @descriptor = { "query" => {}, "headers" => {} }
  end

  def query(values)
    @descriptor["query"] = @descriptor["query"].merge(BehaviorBoundary.copy(values, 4096))
    self
  end

  def header(key, value)
    @descriptor["headers"][key] = value
    self
  end

  def body(value)
    BehaviorBoundary.invalid("HTTP reads cannot contain a request body.") if ["GET", "HEAD"].include?(@method)
    @descriptor["body"] = value
    self
  end

  def retrieve
    @context.rest_exchange(@name, @method, @path, @descriptor)
  end
end

class BehaviorHostCache
  def initialize(name)
    @name = name
  end

  def get(key, _execution)
    BehaviorStorage.ask({ "op" => "cache.get", "cache" => @name, "key" => key })
  end

  def set(key, value, execution)
    BehaviorStorage.ask({ "op" => "cache.set", "cache" => @name, "key" => key, "value" => value,
      "expiresAt" => execution.expires_at, "maxEntries" => execution.policy["maxEntries"] })
  end

  def delete(key, _execution)
    BehaviorStorage.ask({ "op" => "cache.delete", "cache" => @name, "key" => key })
  end
end

class BehaviorHostEntities
  def execute(command, _execution)
    BehaviorStorage.ask({ "op" => "entity", "command" => command })
  end
end

class BehaviorStorage
  def self.adapter(group, name)
    return nil unless Behaviors.environment == "tac" && Object.const_defined?(:TacStorage)
    return BehaviorHostCache.new(name) if group == "caches"
    BehaviorHostEntities.new if group == "entities" && name == "indexeddb"
  end

  def self.ask(request)
    answer = TacStorage.ask(request)
    raise BehaviorError.new("storage", "Tachyon storage did not answer.") unless answer.is_a?(Hash)
    raise BehaviorError.new("storage", answer["error"]) if answer["error"].is_a?(String)
    answer["value"]
  end
end

class BehaviorRepository
  def initialize(context, record)
    @context = context
    @record = record
  end

  def find(page = {})
    rules = { "defaultSize" => 20, "maxSize" => 100, "sort" => [@record["entity"]["id"]] }
    @context.entity_operation(@record, "find", BehaviorBoundary.page(page, rules))
  end

  def save(value)
    @context.entity_operation(@record, "save", value)
  end

  def delete(id)
    unsafe = (id.is_a?(String) && (id.empty? || id.bytesize > 256)) || (id.is_a?(Integer) && id.abs > BehaviorBoundary::MAX_SAFE_INTEGER)
    BehaviorBoundary.invalid("Invalid entity identifier.") if unsafe
    @context.entity_operation(@record, "delete", id)
  end
end

class BehaviorValidation
  BLANK = /\A[\u0009-\u000D\u0020\u0085\u00A0\u1680\u2000-\u200A\u2028\u2029\u202F\u205F\u3000]*\z/

  def self.error(path)
    raise BehaviorError.new("validation", "Behavior validation failed.", path)
  end

  def self.apply(schema, input)
    value = begin
      BehaviorBoundary.copy(input)
    rescue StandardError
      error("$.json")
    end
    error("$.object") unless value.is_a?(Hash)
    error("$.schema") unless schema.is_a?(Hash) && schema.key?("fields")
    schema["fields"].each { |field, rules| check(field, rules, value[field]) }
    value
  end

  def self.check(field, rules, data)
    error("#{field}.notNull") if rules["notNull"] && data.nil?
    error("#{field}.notBlank") if rules["notBlank"] && (!data.is_a?(String) || data.match?(BLANK))
    return if data.nil?
    if rules.key?("size")
      size = data.is_a?(String) || data.is_a?(Array) || data.is_a?(Hash) ? data.size : -1
      error("#{field}.size") if size < rules["size"]["min"] || size > rules["size"]["max"]
    end
    number = data.is_a?(Integer) || data.is_a?(Float)
    error("#{field}.min") if rules.key?("min") && (!number || data < rules["min"])
    error("#{field}.max") if rules.key?("max") && (!number || data > rules["max"])
  end
end

class BehaviorProperties
  WHOLE = /\A-?(0|[1-9][0-9]*)\z/
  NUMBER = /\A-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?\z/

  def self.error(code)
    raise BehaviorError.new(code, "Invalid or inactive behavior configuration.")
  end

  def self.source(input, code)
    values = BehaviorBoundary.copy(input, 65_536)
    if values.is_a?(Hash) && (values.key?("tac") || values.key?("yon"))
      prefix = "#{Behaviors.environment}."
      normalized = {}
      application(values).each do |path, value|
        normalized[path.upcase.tr(".", "_")] = value if path.start_with?(prefix)
      end
      values = normalized
    end
    error(code) unless values.is_a?(Hash) && values.size <= 256
    prefix = Behaviors.environment == "tac" ? "TAC_" : "YON_"
    values.each do |key, value|
      valid = key.bytesize <= 256 && key.start_with?(prefix) && key.match?(/\A[A-Z][A-Z0-9_]*\z/) &&
        value.is_a?(String) && value.bytesize <= 4096
      error(code) unless valid
    end
    values
  rescue BehaviorError => failure
    raise failure if failure.behavior_code == code
    error(code)
  end

  def self.select(record, options)
    options = BehaviorBoundary.named(options)
    profiles = options["profiles"] || []
    error("configuration") unless profiles.is_a?(Array) && profiles.size <= 32 && profiles.uniq.size == profiles.size
    profiles.each { |name| error("configuration") unless name.is_a?(String) && name.match?(/\A[A-Za-z_$][A-Za-z0-9_$]{0,63}\z/) }
    values = source(options["properties"] || {}, "configuration")
    error("inactive-configuration") if record.key?("profiles") && (profiles & record["profiles"]).empty?
    if record.key?("conditionalOnProperty")
      condition = record["conditionalOnProperty"]
      error("inactive-configuration") if values.keys.any? { |key| key.start_with?("#{condition['name']}_") }
      matches = values.key?(condition["name"]) ? values[condition["name"]] == condition["havingValue"] : condition["matchIfMissing"]
      error("inactive-configuration") unless matches
    end
    options.merge("profiles" => profiles, "properties" => values)
  end

  def self.bind(policy, input)
    error("configuration-properties") if policy.nil?
    values = source(input, "configuration-properties")
    names = policy["fields"].values.map { |descriptor| policy["prefix"] + descriptor["name"] }
    values.each_key { |name| error("configuration-properties") if name.start_with?(policy["prefix"]) && !names.include?(name) }
    result = {}
    policy["fields"].each { |field, descriptor| result[field] = value(descriptor, values, policy["prefix"] + descriptor["name"]) }
    BehaviorBoundary.copy(result, 65_536)
  end

  def self.application(input)
    tree = BehaviorBoundary.copy(input, 65_536)
    sections = tree.is_a?(Hash) ? tree.select { |key, _| key == "tac" || key == "yon" } : {}
    error("configuration-properties") if sections.empty?
    flat = {}
    sections.each do |root, value|
      error("configuration-properties") unless value.is_a?(Hash)
      flatten(value, root, 0, flat)
    end
    flat
  rescue BehaviorError
    error("configuration-properties")
  end

  def self.flatten(value, path, depth, flat)
    error("configuration-properties") if depth > 16 || path.bytesize > 256 || flat.size >= 256
    error("configuration-properties") if path == "yon.server.port" && !value.is_a?(Integer)
    if value.is_a?(Hash)
      return flat["#{path}."] = "" if depth > 0 && value.empty?
      value.each do |key, child|
        error("configuration-properties") unless key.is_a?(String) && key.match?(/\A[a-z][a-z0-9]*\z/)
        flatten(child, "#{path}.#{key}", depth + 1, flat)
      end
    elsif value.is_a?(Array) || value.nil?
      flat["#{path}."] = ""
    else
      error("configuration-properties") unless value.is_a?(String) || value == true || value == false || value.is_a?(Integer) || value.is_a?(Float)
      text = value.to_s
      error("configuration-properties") if text.bytesize > 4096 || flat.size >= 256
      flat[path] = text
    end
  end

  def self.values(policy, input)
    error("configuration-properties") if policy.nil?
    flat = application(input)
    result = {}
    policy.each { |field, descriptor| result[field] = value(descriptor, flat, descriptor["path"]) }
    BehaviorBoundary.copy(result, 65_536)
  end

  def self.value(descriptor, values, name)
    type = descriptor["type"]
    error("configuration-properties") if values.keys.any? { |path| path.start_with?("#{name}.") }
    value = if !values.key?(name)
      error("configuration-properties") unless descriptor.key?("default")
      descriptor["default"]
    elsif type == "string"
      values[name]
    elsif type == "boolean"
      error("configuration-properties") unless ["true", "false"].include?(values[name])
      values[name] == "true"
    else
      error("configuration-properties") unless values[name].match?(type == "integer" ? WHOLE : NUMBER)
      values[name].to_f
    end
    error("configuration-properties") if type == "string" && !(value.is_a?(String) && value.bytesize <= 4096)
    ["integer", "number"].include?(type) ? number(descriptor, type, value) : value
  end

  def self.number(descriptor, type, value)
    valid = (value.is_a?(Integer) || value.is_a?(Float)) && value.to_f.finite? && value.abs <= BehaviorBoundary::MAX_SAFE_INTEGER &&
      (type != "integer" || value.to_f.floor == value) &&
      (!descriptor.key?("min") || value >= descriptor["min"]) && (!descriptor.key?("max") || value <= descriptor["max"])
    error("configuration-properties") unless valid
    type == "integer" ? value.to_i : value
  end
end

class Behaviors
  @records = {}
  @setups = {}
  @environment = "yon"

  def self.environment
    @environment
  end

  def self.records
    @records
  end

  def self.register(records_json, environment, module_id)
    @environment = environment
    JSON.parse(records_json).each do |name, record|
      record["module"] = module_id
      @records[name] = record
    end
  end

  def self.create(setup, options = {}, **named_options)
    name = BehaviorBoundary.short_name(setup)
    record = @records[name]
    BehaviorBoundary.invalid("Setup must be source-local.") unless record && record["configuration"]
    options = BehaviorProperties.select(record, BehaviorBoundary.named(options).merge(BehaviorBoundary.named(named_options)))
    context = BehaviorContext.new(@records, options)
    begin
      setup_key = [name, options["profiles"].sort, options["properties"].sort].inspect
      shared = @setups[setup_key] ||= { "adapters" => configured_adapters(context, setup), "users" => 0 }
      shared["users"] += 1
      context.share(setup_key, shared["adapters"])
      context
    rescue StandardError
      context.dispose
      raise
    end
  end

  def self.configured_adapters(context, setup)
    context.configure(lambda { |execution| setup.configure(execution) })
    context.adapters
  end

  def self.release_setup(setup_key)
    shared = @setups[setup_key]
    return unless shared
    shared["users"] -= 1
    @setups.delete(setup_key) if shared["users"].zero?
  end

  def self.construct(type, values = {}, **named_values)
    values = BehaviorBoundary.named(values).merge(BehaviorBoundary.named(named_values))
    instance = type.new
    values.each do |field, value|
      BehaviorBoundary.invalid("Constructor fields must be plain field names.") unless field.match?(/\A[a-z_][A-Za-z0-9_]*\z/)
      instance.instance_variable_set("@#{field}", value)
    end
    instance
  end

  def self.construct_with(type, context, fakes)
    BehaviorBoundary.invalid("A test unit constructs roles with its own behavior context.") unless context.is_a?(BehaviorContext)
    instance = type.new
    fakes.each do |name, fake|
      BehaviorBoundary.invalid("Fake '#{name}' names no field on this type.") unless instance.respond_to?(name)
      instance.instance_variable_set("@#{name}", fake) unless fake.nil?
    end
    instance.instance_variable_set(:@behaviors, context)
    instance
  end

  def self.properties(type, values)
    BehaviorProperties.bind((@records[BehaviorBoundary.short_name(type)] || {})["configurationProperties"], values)
  end

  def self.values(type, application)
    BehaviorProperties.values((@records[BehaviorBoundary.short_name(type)] || {})["values"], application)
  end

  def self.validate(type, value)
    BehaviorValidation.apply((@records[BehaviorBoundary.short_name(type)] || {})["validated"], value)
  end
end

class BehaviorCalls
  @keys = []

  class << self
    def record(key)
      @keys << key unless @keys.include?(key)
    end

    def take
      keys = @keys.dup
      @keys.clear
      keys
    end
  end
end
end
