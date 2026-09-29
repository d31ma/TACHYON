#[Description("Behavior conformance HandlerChecks")]
class HandlerChecks
  def self.behavior_error(message)
    yield
  rescue BehaviorError => error
    error
  else
    raise message
  end

  def self.check_configuration(values)
    settings = Behaviors.properties(Settings, values)
    raise "typed configuration" unless settings["size"] == 25 && settings["enabled"] == true
    namespaced = Behaviors.properties(Settings, { "yon" => { "catalog" => { "size" => 25, "enabled" => true } } })
    raise "namespaced binding" unless namespaced == settings
    source = { "yon" => { "server" => { "host" => "127.0.0.1", "port" => 9000 }, "catalog" => { "size" => 25 } } }
    application = Behaviors.values(ApplicationValues, source)
    raise "application values" unless application["port"] == 9000
    raise "application values" unless application["size"] == 25
    raise "application values" unless application["redis"] == "redis://127.0.0.1:6379"
  end

  def self.check_value_limits
    ["x" * 4096, "é" * 2048].each do |text|
      bound = Behaviors.values(ApplicationValues, { "yon" => { "redis" => { "url" => text } } })
      raise "bounded selected value" unless bound["redis"] == text
    end
    ["x" * 4097, "é" * 2049].each do |text|
      error = behavior_error("oversized selected value must fail") do
        Behaviors.values(ApplicationValues, { "yon" => { "redis" => { "url" => text } } })
      end
      raise error unless error.behavior_code == "configuration-properties"
      raise error if error.message.include?(text)
    end
    error = behavior_error("oversized scalar accepted") do
      Behaviors.values(ApplicationValues, { "yon" => { "datasource" => { "password" => "x" * 4097 } } })
    end
    raise error unless error.behavior_code == "configuration-properties"
    error = behavior_error("a string port must fail") do
      Behaviors.values(ApplicationValues, { "server" => { "port" => "9000" } })
    end
    raise error unless error.behavior_code == "configuration-properties"
  end

  def self.check_validation
    accepted = ValidationService.new.accept({ "name" => "éé", "price" => 20 })
    raise "validation" unless accepted == { "name" => "éé", "price" => 20 }
    error = behavior_error("must fail") do
      Behaviors.validate(ProductInput, { "name" => "\u0085\u00a0" })
    end
    raise error unless error.behavior_code == "validation" && error.path == "name.notBlank"
  end

  def self.check_inactive_configuration(signal)
    error = behavior_error("must be inactive") do
      Behaviors.create(AdaptersSetup, partition: "catalog", signal: signal)
    end
    raise error unless error.behavior_code == "inactive-configuration"
  end

  def self.exercise_cache(catalog, context, page)
    first = catalog.read(page)
    cached = catalog.read(page)
    context.evict(catalog, "read", [page])
    fresh = catalog.read(page)
    catalog.replace(99, page)
    raise "cache put" unless catalog.read(page)["calls"] == 99
    begin
      catalog.replace("fail", page)
      raise ArgumentError, "must fail"
    rescue RuntimeError => error
      raise unless error.message == "failed mutation"
    end
    raise "failed put changed cache" unless catalog.read(page)["calls"] == 99
    catalog.remove(page)
    raise "cache eviction" unless catalog.read(page)["calls"] == 3
    [first, cached, fresh]
  end

  def self.check_resilience(context)
    resilient = Behaviors.construct(Resilient, behaviors: context)
    raise "retry contract" unless resilient.flaky == 3 && Resilient.attempts == 3
    raise "limit contract" unless resilient.bounded("ok") == "ok"
  end

  def self.check_queries(context)
    products = Behaviors.construct(ProductsStore, behaviors: context)
    raise "query results" unless products.find_by_status("active") == [{ "id" => "one", "status" => "active" }]
    raise "query results" unless products.count_by_status("active") == 1
    raise "query results" unless products.available("active") == true
  end

  def self.check_mapping(context)
    summaries = Behaviors.construct(ProductSummaries, behaviors: context)
    summary = summaries.to_summary({ "detail" => { "id" => 7 }, "name" => "Anvil", "extra" => "dropped" })
    expected = { "absent" => nil, "kind" => "product", "productId" => 7, "title" => "Anvil" }
    raise "mapping fields #{JSON.generate(summary)}" unless summary == expected && summary.keys == expected.keys
    refused = begin
      summaries.to_summary(["not an object"])
      false
    rescue BehaviorError => error
      error.behavior_code == "invalid"
    end
    raise "a mapping reads one JSON object argument" unless refused
  end

  def self.cancelled?(catalog)
    catalog.remote
    false
  rescue BehaviorError => error
    error.behavior_code == "cancelled"
  end

  def self.exercise(context, signal)
    catalog = CatalogService.new
    catalog.behaviors = context
    client = CatalogClient.new
    client.instance_variable_set(:@behaviors, context)
    page = { "page" => 2, "size" => 5 }
    first, cached, fresh = exercise_cache(catalog, context, page)
    remote = client.remote
    repository = context.repository(ProductEntity)
    raise "cached exchange" unless client.remote == remote && Backend.exchanges == 1
    raise "path variable" unless client.find(7) == { "path" => "/products/7" }
    saved = repository.save({ "id" => "one" })
    found = repository.find
    deleted = repository.delete("one")
    check_resilience(context)
    check_queries(context)
    check_mapping(context)
    signal.abort
    {
      "first" => first,
      "cached" => cached,
      "fresh" => fresh,
      "remote" => remote,
      "saved" => saved,
      "found" => found,
      "deleted" => deleted,
      "cancelled" => cancelled?(client),
      "authorizations" => Backend.authorizations
    }
  end
end
