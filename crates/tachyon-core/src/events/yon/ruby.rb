class ApplicationEventPublisher
  def publish_event(value, retain = false)
    record = value.is_a?(Struct) || (defined?(Data) && value.is_a?(Data))
    unless record
      STDERR.write("Yon local event publication failed.\n")
      return value
    end
    YonEvents.publish(value.class.name.to_s.split("::").last, YonModels.plain(value), retain == true)
    value
  end
end

class YonEvents
  MAX_EVENT_BYTES = 65_536
  MAX_QUEUED_EVENTS = 128
  MAX_LISTENERS = 1024
  MAX_DELIVERIES = 4096
  MAX_REPUBLISHED_EVENTS = 32

  @active = nil
  @topics = []

  class << self
    attr_accessor :active, :topics
  end

  attr_reader :destroyers

  def initialize(only = nil)
    raise "Nested local event invocation" unless YonEvents.active.nil?
    @only = only
    @outbound = []
    @queue = []
    @retained = {}
    @listeners = []
    @destroyers = []
    @closed = false
    @deliveries = 0
    YonEvents.active = self
  end

  def self.publish(name, value, retain)
    scope = active
    raise "Local event owner is disposed" if scope.nil? || scope.closed?
    scope.enqueue(name, JSON.generate(value), retain)
  rescue StandardError
    STDERR.write("Yon local event publication failed.\n")
  end

  def closed?
    @closed
  end

  def enqueue(name, encoded, retain)
    raise "Local event capacity exceeded" if encoded.bytesize > MAX_EVENT_BYTES || @queue.size >= MAX_QUEUED_EVENTS
    @retained[name] = encoded if retain
    @queue << [name, encoded]
    @outbound << [name, encoded] if YonEvents.topics.include?(name)
  end

  def receive(name, encoded)
    @queue << [name, encoded]
  end

  def listen(options, callback, replay, key = "")
    return if @only && !@only.include?(key)
    options = JSON.parse(options)
    options["events"].each do |name|
      raise "Local listener capacity exceeded" if @listeners.size >= MAX_LISTENERS
      @listeners << {
        "name" => name, "callback" => callback, "order" => options["order"].to_i,
        "condition" => options["condition"], "models" => options["models"],
        "returns" => options["returns"], "background" => options["async"] == true,
        "sequence" => @listeners.size
      }
      @queue << [name, @retained[name]] if replay && @retained.key?(name)
    end
    @listeners.sort_by! { |listener| [listener["order"], listener["sequence"]] }
  end

  def drain
    until @queue.empty?
      name, encoded = @queue.shift
      @deliveries += 1
      if @deliveries > MAX_DELIVERIES
        @queue.clear
        STDERR.write("Yon local event delivery bound exceeded.\n")
        return
      end
      deliver_pass(name, encoded)
    end
    send_topics
  end

  def send_topics
    until @outbound.empty?
      name, encoded = @outbound.shift
      begin
        YonResources.call({ "operation" => "topic", "topic" => name, "payload_json" => encoded })
      rescue StandardError
        STDERR.write("Yon topic publication failed.\n")
      end
    end
  end

  def close
    return if @closed
    @closed = true
    @queue.clear
    @listeners.clear
    @destroyers.reverse_each do |destroy|
      destroy.call
    rescue StandardError
      STDERR.write("Yon local lifecycle cleanup failed.\n")
    end
    @retained.clear
    YonEvents.active = nil
  end

  private

  def deliver_pass(name, encoded)
    listeners = @listeners.select { |listener| listener["name"] == name }
    background, foreground = listeners.partition { |listener| listener["background"] }
    foreground.each { |listener| deliver(listener, name, encoded) }
    background.each { |listener| deliver(listener, name, encoded) }
  end

  def deliver(listener, name, encoded)
    value = JSON.parse(encoded)
    if listener["models"] && !YonEventModels.fits?(listener["models"], name, value)
      STDERR.write("Yon local event payload does not fit the listener model.\n")
      return
    end
    return if listener["condition"] && !YonEventConditions.matches?(listener["condition"], value, name)
    result = listener["callback"].call(value)
    republish(result, listener["returns"]) unless listener["background"]
  rescue StandardError
    STDERR.write("Yon local event listener failed.\n")
  end

  def republish(result, returns)
    return if returns.nil?
    items = result.is_a?(Array) ? result.first(MAX_REPUBLISHED_EVENTS) : [result]
    items.each { |item| YonEvents.publish(returns, YonModels.plain(item), false) unless item.nil? }
  end
end

class YonEventModels
  def self.fits?(models, shape, value)
    return value.nil? || fits?(models, shape[1..], value) if shape.start_with?("?")
    return value.is_a?(Array) && value.all? { |item| fits?(models, shape[2..], item) } if shape.start_with?("[]")
    case shape
    when "any" then true
    when "string" then value.is_a?(String)
    when "integer" then value.is_a?(Integer)
    when "number" then value.is_a?(Integer) || value.is_a?(Float)
    when "boolean" then value == true || value == false
    else fits_model?(models, models[shape], value)
    end
  end

  def self.fits_model?(models, model, value)
    return false if model.nil? || !value.is_a?(Hash)
    fields = model["fields"]
    return false if !model["open"] && value.keys.any? { |key| !fields.key?(key) }
    fields.all? do |name, field|
      required = field.start_with?("!")
      next !required unless value.key?(name)
      fits?(models, required ? field[1..] : field, value[name])
    end
  end
end

class YonEventConditions
  def self.matches?(node, value, event)
    test(node, value, event)
  rescue StandardError
    false
  end

  def self.test(node, value, event)
    case node["k"]
    when "and" then test(node["l"], value, event) && test(node["r"], value, event)
    when "or" then test(node["l"], value, event) || test(node["r"], value, event)
    when "not" then !test(node["e"], value, event)
    when "truthy" then truthy?(operand(node["e"], value, event))
    when "cmp" then compare(node, value, event)
    else false
    end
  end

  def self.operand(node, value, event)
    return node["v"] if node["k"] == "lit"
    return event if node["k"] == "event"
    (node["path"] || []).reduce(value) do |current, segment|
      return nil unless current.is_a?(Hash) && current.key?(segment)
      current[segment]
    end
  end

  def self.truthy?(value)
    return false if value.nil? || value == false || value == 0 || value == ""
    !(value.is_a?(Float) && value.nan?)
  end

  def self.compare(node, value, event)
    left = operand(node["l"], value, event)
    right = operand(node["r"], value, event)
    numbers = number?(left) && number?(right)
    strings = left.is_a?(String) && right.is_a?(String)
    if node["op"] == "eq" || node["op"] == "ne"
      equal = numbers ? left == right : ((strings || left == true || left == false || left.nil?) && left == right && left.class == right.class)
      return (node["op"] == "eq") == equal
    end
    return false unless numbers || strings
    order = left <=> right
    { "lt" => order.negative?, "le" => order <= 0, "gt" => order.positive? }.fetch(node["op"], order >= 0)
  end

  def self.number?(value)
    value.is_a?(Integer) || value.is_a?(Float)
  end
end
