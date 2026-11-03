#[Description("Behavior conformance Backend")]
class Backend
  @entries = {}
  @calls = 0
  @prices = 0
  @exchanges = 0
  @authorizations = 0

  class << self
    attr_accessor :entries, :calls, :prices, :exchanges, :authorizations
  end

  def self.authorize(call, execution)
    self.authorizations += 1
    return false unless execution.partition == "catalog"
    return true if call["method"] == "remote"
    return call["keyArguments"].size == 1 if call["method"] == "price"
    call["keyArguments"][0]["size"] == 5
  end

  def self.get(key, execution)
    entries.fetch(key, { "hit" => false })
  end

  def self.set(key, value, execution)
    entries[key] = { "hit" => true, "value" => value, "expiresAt" => execution.expires_at }
  end

  def self.delete(key, execution)
    entries.delete(key)
  end

  def self.exchange(request, execution)
    self.exchanges += 1
    raise "transport policy" unless request["redirect"] == "error"
    raise "transport policy" unless request["credentials"] == "omit"
    raise "transport policy" unless request["maxResponseBytes"] == 1_048_576
    { "path" => request["path"] }
  end

  def self.execute(command, execution)
    return command unless command["operation"] == "query"
    query = command["query"]
    return query["where"].size if query["kind"] == "delete"
    return [{ "id" => "one", "status" => "active" }] if query["where"].empty?
    return [{ "id" => query["where"][0]["value"], "status" => "active" }] if query["where"][0]["field"] == "id"
    raise "query binding" unless query["where"][0]["value"] == "active"
    return 1 if query["kind"] == "count"
    return true if query["kind"] == "exists"
    [{ "id" => "one", "status" => "active" }]
  end

  def self.authorize_entity(command, execution)
    command["entity"] == "ProductEntity" && execution.partition == "catalog"
  end
end
