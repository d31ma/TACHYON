require "json"

class Service
  attr_accessor :behaviors
end

class Orders
  attr_accessor :behaviors
end

class Ledger
  attr_accessor :behaviors
end

class NotesSetup
  BODY = "body"

  class << self
    attr_accessor :configured
  end

  def self.configure(_execution)
    self.configured += 1
    notes = ["Ada"]
    exchange = lambda do |request, _execution|
      notes << request[BODY] if request.key?(BODY)
      { "items" => notes.dup }
    end
    { "http" => { "notes" => { "exchange" => exchange } } }
  end
end
NotesSetup.configured = 0

class RuntimeContract
  def self.load_runtime
    unless defined?(BehaviorContext)
      path = File.expand_path("../../../src/behavior/polyglot/ruby_runtime.rb", __dir__)
      Object.class_eval(File.read(path), path)
    end
    Behaviors.register("{}", "yon", "a" * 64)
  end

  def initialize
    @failures = []
    @state = { "hit" => { "hit" => false }, "authorization" => 0, "calls" => 0, "saved" => 0 }
    @attempts = 0
    @log = []
    @sequence = 0
    @seen = []
  end

  def reject_behavior(label, code = "invalid")
    yield
    @failures << "#{label}: unexpectedly accepted"
  rescue BehaviorError => error
    @failures << "#{label}: wrong code #{error.behavior_code}" unless error.behavior_code == code
  rescue StandardError => error
    @failures << "#{label}: leaked #{error.class}"
  end

  def service_context(signal, entity)
    record = {
      "name" => "Service",
      "module" => "a" * 64,
      "cache" => { "name" => "data", "ttlMs" => 1000, "maxBytes" => 1024, "maxEntries" => 10 },
      "methods" => {
        "read" => { "cacheable" => { "keyArgs" => [0] } },
        "remote" => { "httpExchange" => { "client" => "api", "method" => "GET", "path" => "/products" } }
      }
    }
    context = BehaviorContext.new({ "Service" => record, "Product" => entity }, { "partition" => "tenant", "signal" => signal })
    context.configure { |_execution| service_adapters }
    context
  end

  def service_adapters
    {
      authorize: lambda do |_call, _execution|
        @state["authorization"] += 1
        true
      end,
      caches: { data: {
        get: ->(_key, _execution) { @state["hit"] },
        set: ->(_key, value, execution) { @state["hit"] = { "hit" => true, "value" => value, "expiresAt" => execution.expires_at } },
        delete: ->(_key, _execution) { @state["hit"] = { "hit" => false } }
      } },
      http: { api: { exchange: lambda do |request, _execution|
        raise "transport contract" unless request["redirect"] == "error" && request["credentials"] == "omit"
        "transport"
      end } },
      entities: { store: { execute: lambda do |command, _execution|
        @state["saved"] += 1
        command["operation"]
      end } }
    }
  end

  def read(context, service)
    context.invoke(service, "Service", "read", [{ "id" => 1 }], lambda do |value|
      @state["calls"] += 1
      value
    end)
  end

  def check_cache(context, service)
    raise "cache/auth contract" unless read(context, service) == { "id" => 1 }
    raise "cache/auth contract" unless read(context, service) == { "id" => 1 }
    raise "cache/auth contract" unless @state["authorization"] == 2 && @state["calls"] == 1
    context.evict(service, "read", [{ "id" => 1 }])
    read(context, service)
    raise "eviction contract" unless @state["calls"] == 2
    @state["hit"] = { "hit" => true, "value" => nil, "expiresAt" => BehaviorClock.now_ms + 100_000 }
    reject_behavior("cache excessive TTL") { read(context, service) }
  end

  def check_http(context, service)
    remote = ->(descriptor) { context.invoke(service, "Service", "remote", [], -> { descriptor }) }
    reject_behavior("GET body") { remote.call({ "body" => {} }) }
    reject_behavior("Cookie header") { remote.call({ "headers" => { "Cookie" => "x" } }) }
    raise "HTTP contract" unless remote.call({}) == "transport"
  end

  def check_repository(context, entity)
    repository = context.repository("Product")
    reject_behavior("save missing id") { repository.save({}) }
    reject_behavior("save unsafe id") { repository.save({ "id" => true }) }
    reject_behavior("unsupported entity operation") { context.entity_operation(entity, "drop-table", {}) }
    reject_behavior("direct entity save bypass") { context.entity_operation(entity, "save", {}) }
    reject_behavior("direct entity find bypass") { context.entity_operation(entity, "find", { "size" => 10_000 }) }
    raise "entity contract" unless repository.save({ "id" => "one" }) == "save"
    raise "entity contract" unless repository.delete("one") == "delete"
  end

  def check_boundaries
    reject_behavior("whitespace partition") { BehaviorContext.new({}, { "partition" => "  ", "signal" => BehaviorSignal.new }) }
    page_rules = { "defaultSize" => 20, "maxSize" => 100, "sort" => [] }
    reject_behavior("null page") { BehaviorBoundary.page({ "page" => nil }, page_rules) }
    reject_behavior("null offset") { BehaviorBoundary.page({ "offset" => nil }, page_rules) }
    reject_behavior("nonfinite JSON") { BehaviorBoundary.copy(Float::INFINITY) }
    reject_behavior("invalid UTF8 JSON") { BehaviorBoundary.copy("\xff".dup.force_encoding("UTF-8")) }
    reject_behavior("large JSON") { BehaviorBoundary.copy("x" * 1_048_577) }
    reject_behavior("unsafe field") { BehaviorBoundary.copy({ "__proto__" => 1 }) }
    optional_schema = { "fields" => { "name" => { "size" => { "min" => 1, "max" => 16 } } } }
    reject_behavior("a list is not a DTO object", "validation") { BehaviorValidation.apply(optional_schema, []) }
    raise "empty DTO object shape lost" unless BehaviorValidation.apply(optional_schema, {}) == {}
    two_bytes = { "fields" => { "name" => { "size" => { "min" => 1, "max" => 2 } } } }
    reject_behavior("Length counts UTF-8 bytes", "validation") { BehaviorValidation.check(two_bytes, { "name" => "\u00e9\u00e9" }) }
    raise "symbol keys are JSON text" unless BehaviorBoundary.copy({ b: 1, a: :x }) == { "a" => "x", "b" => 1 }
  end

  def resilient_context
    resilient = { "name" => "Orders", "methods" => {
      "flaky" => { "retryable" => { "maxRetries" => 3, "delayMs" => 0, "multiplier" => 1, "maxDelayMs" => 100, "jitterMs" => 0 } },
      "guarded" => { "concurrencyLimit" => { "limit" => 1, "policy" => "reject" } }
    } }
    context = BehaviorContext.new({ "Orders" => resilient }, { "partition" => "tenant", "signal" => BehaviorSignal.new })
    context.configure { |_execution| {} }
    context
  end

  def check_retries(context, orders)
    @attempts = 0
    flaky = context.invoke(orders, "Orders", "flaky", [], lambda do
      @attempts += 1
      raise "transient" if @attempts < 3
      "ok"
    end)
    raise "retry contract: #{@attempts} attempts" unless flaky == "ok" && @attempts == 3
    @attempts = 0
    begin
      context.invoke(orders, "Orders", "flaky", [], lambda do
        @attempts += 1
        raise "permanent"
      end)
      @failures << "exhausted retries: unexpectedly accepted"
    rescue RuntimeError
      nil
    end
    raise "retry bound: #{@attempts} attempts" unless @attempts == 4
  end

  def check_concurrency_limit(context, orders)
    nested = -> { context.invoke(orders, "Orders", "guarded", [], -> { "nested" }) }
    reject_behavior("re-entrant concurrency limit", "capacity") { context.invoke(orders, "Orders", "guarded", [], nested) }
    free = context.invoke(orders, "Orders", "guarded", [], -> { "free" })
    raise "limit contract: slot not released" unless free == "free"
  end

  def check_resilience
    context = resilient_context
    orders = Orders.new
    orders.behaviors = context
    check_retries(context, orders)
    check_concurrency_limit(context, orders)
  end

  def execute_ledger(command)
    unless command["operation"] == "transaction"
      @log << "#{command['operation']}@#{command['transaction'] || 'none'}"
      return { "id" => "one" }
    end
    entry = command["action"]
    entry += ":#{command['transaction']}" if command.key?("transaction")
    @log << entry
    return { "ok" => true } unless command["action"] == "begin"
    @sequence += 1
    { "transaction" => "t#{@sequence}" }
  end

  def ledger_context
    transactional = { "name" => "Ledger", "methods" => { "post" => { "transactional" => { "adapter" => "ledger" } } } }
    ledger_entity = { "name" => "Entry", "entity" => { "adapter" => "ledger", "id" => "id" }, "table" => "entries", "methods" => {} }
    context = BehaviorContext.new({ "Ledger" => transactional, "Entry" => ledger_entity }, { "partition" => "tenant", "signal" => BehaviorSignal.new })
    context.configure { |_execution| { entities: { ledger: { execute: ->(command, _execution) { execute_ledger(command) } } } } }
    context
  end

  def post(context, ledger, fail)
    context.invoke(ledger, "Ledger", "post", [], lambda do
      context.repository("Entry").save({ "id" => "one" })
      raise "domain rejected the entry" if fail
      "posted"
    end)
  end

  def check_transactions
    @log = []
    @sequence = 0
    context = ledger_context
    ledger = Ledger.new
    ledger.behaviors = context
    raise "transaction contract: #{@log.join(',')}" unless post(context, ledger, false) == "posted"
    raise "transaction contract: #{@log.join(',')}" unless @log == ["begin", "save@t1", "commit:t1"]
    @log = []
    begin
      post(context, ledger, true)
      @failures << "a failing transactional body was accepted"
    rescue RuntimeError
      nil
    end
    raise "rollback contract: #{@log.join(',')}" unless @log == ["begin", "save@t2", "rollback:t2"]
    @log = []
    context.repository("Entry").save({ "id" => "two" })
    raise "transaction leaked past its method: #{@log.join(',')}" unless @log == ["save@none"]
  end

  def execute_joined(command)
    if command["operation"] == "transaction"
      execute_ledger(command)
    else
      @log << "#{command['operation']}@#{command['transaction'] || 'none'}"
      raise "the second write failed" if command["value"]["id"] == "broken"

      { "id" => command["value"]["id"] }
    end
  end

  def joined_context(records, partition)
    context = BehaviorContext.new(records, { "partition" => partition, "signal" => BehaviorSignal.new })
    adapter = { execute: ->(command, _execution) { execute_joined(command) } }
    context.configure { |_execution| { entities: { ledger: adapter, audit: adapter } } }
    context
  end

  def settle(service, store, second)
    ledger = Ledger.new
    ledger.behaviors = service
    service.invoke(ledger, "Ledger", "post", [], lambda do
      store.repository("Entry").save({ "id" => "first" })
      store.repository("Note").save({ "id" => "note" })
      store.repository("Entry").save({ "id" => second })
      "settled"
    end)
  end

  def check_joined_transactions
    @log = []
    @sequence = 0
    service = joined_context({ "Ledger" => { "name" => "Ledger", "methods" => { "post" => { "transactional" => { "adapter" => "ledger" } } } } }, "ledger")
    entry = { "name" => "Entry", "entity" => { "adapter" => "ledger", "id" => "id" }, "table" => "entries", "methods" => {} }
    note = { "name" => "Note", "entity" => { "adapter" => "audit", "id" => "id" }, "table" => "notes", "methods" => {} }
    store = joined_context({ "Entry" => entry, "Note" => note }, "entries")
    raise "joined transaction: #{@log.join(',')}" unless settle(service, store, "second") == "settled"
    raise "joined transaction: #{@log.join(',')}" unless @log == ["begin", "save@t1", "save@none", "save@t1", "commit:t1"]
    @log = []
    begin
      settle(service, store, "broken")
      @failures << "a failed second write was accepted"
    rescue StandardError
      nil
    end
    raise "joined rollback: #{@log.join(',')}" unless @log == ["begin", "save@t2", "save@none", "save@t2", "rollback:t2"]
  end

  def rest_context
    context = BehaviorContext.new({}, { "partition" => "tenant", "signal" => BehaviorSignal.new })
    exchange = lambda do |request, _execution|
      raise "transport contract" unless request["redirect"] == "error" && request["credentials"] == "omit"
      @seen << "#{request['method']} #{request['path']}"
      { "ok" => true, "body" => request["body"] }
    end
    context.configure { |_execution| { http: { catalog: { exchange: exchange } } } }
    context
  end

  def check_rest_client
    context = rest_context
    rest = context.rest_client("catalog")
    read = rest.get("/products").query({ "page" => 2 }).header("X-Test", "present").retrieve
    raise "RestClient read" unless read == { "body" => nil, "ok" => true }
    posted = rest.post("/products").body({ "sku" => "anvil" }).retrieve
    raise "RestClient write" unless posted["body"] == { "sku" => "anvil" }
    raise "RestClient transport: #{@seen.join(',')}" unless @seen == ["GET /products", "POST /products"]
    reject_behavior("RestClient query path") { rest.get("/products?page=1") }
    reject_behavior("RestClient traversal") { rest.get("/../secrets") }
    reject_behavior("RestClient read body") { rest.get("/products").body({}) }
    reject_behavior("RestClient adapter name") { context.rest_client("not an identifier") }
    reject_behavior("RestClient transport header") { rest.get("/products").header("Host", "attacker.example").retrieve }
  end

  def check_cancellation(signal, context, service)
    signal.abort
    reject_behavior("owner abort", "cancelled") { read(context, service) }
    owner = BehaviorSignal.new
    child = BehaviorSignal.new
    execution = BehaviorExecution.new("tenant", "yon", child, owner, 1000)
    owner.abort
    @failures << "adapter cancellation view does not observe owner abort" unless execution.signal.aborted?
  end

  def create_shared(partition)
    Behaviors.create(NotesSetup, partition: partition, signal: BehaviorSignal.new)
  end

  def check_shared_setup
    Behaviors.register('{"NotesSetup":{"configuration":true,"methods":{}}}', "yon", "a" * 64)
    writer = create_shared("writer")
    reader = create_shared("reader")
    @failures << "two contexts from one Setup configure it more than once" unless NotesSetup.configured == 1
    writer.rest_client("notes").post("/notes").body("Grace").retrieve
    answer = reader.rest_client("notes").get("/notes").retrieve
    @failures << "two contexts from one Setup do not share its adapters" unless answer == { "items" => ["Ada", "Grace"] }
    writer.dispose
    writer.dispose
    later = create_shared("later")
    @failures << "a Setup is configured again while one of its contexts is open" unless NotesSetup.configured == 1
    reader.dispose
    later.dispose
    create_shared("fresh").dispose
    @failures << "a Setup is not configured again after its last context is disposed" unless NotesSetup.configured == 2
  end

  def run
    entity = { "name" => "Product", "entity" => { "adapter" => "store", "id" => "id" }, "table" => "products", "methods" => {} }
    signal = BehaviorSignal.new
    context = service_context(signal, entity)
    service = Service.new
    service.behaviors = context
    check_cache(context, service)
    check_http(context, service)
    check_repository(context, entity)
    check_boundaries
    check_resilience
    check_transactions
    check_joined_transactions
    check_rest_client
    check_shared_setup
    check_cancellation(signal, context, service)
    raise @failures.join("\n") unless @failures.empty?
    puts "Ruby behavior boundary contracts passed"
  end
end

RuntimeContract.load_runtime
RuntimeContract.new.run
