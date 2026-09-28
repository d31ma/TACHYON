class ApplicationEventPublisher
  def publish_event(value, retain = false)
    name = TacValues.model_name(value)
    return TacServices.event_failure if name.nil?
    TacServices.publish(name, value, retain)
    value
  end
end

class TacAnswer
  def self.value(value)
    { "value" => TacValues.plain(value) }
  end

  def self.failure(message)
    { "error" => message.to_s }
  end
end

class TacStorage
  ASK = "tachyon-storage:ask\n"
  TAKE = "tachyon-storage:take\n"
  ROOM = 4096

  def self.ask(request)
    answer = exchange(ASK + JSON.generate(request) + " " * ROOM)
    return answer unless answer.is_a?(Hash) && answer["more"].is_a?(Integer)
    exchange(TAKE + " " * answer["more"])
  end

  def self.exchange(buffer)
    TachyonWeb.host_emit(buffer)
    JSON.parse(buffer)
  end
end

class TacValues
  def self.plain(value)
    return plain_hash(value.to_h) if value.is_a?(Struct)
    return plain_hash(value.to_h) if data?(value)
    return plain_hash(value) if value.is_a?(Hash)
    return value.map { |item| plain(item) } if value.is_a?(Array)
    return value.to_s if value.is_a?(Symbol)
    value
  end

  def self.data?(value)
    Object.const_defined?(:Data) && value.is_a?(Object.const_get(:Data))
  end

  def self.model_name(value)
    return nil unless value.is_a?(Struct) || data?(value)
    value.class.name.to_s.split("::").last
  end

  def self.model(name, value)
    return value unless value.is_a?(Hash) && name.is_a?(String) && Object.const_defined?(name)
    type = Object.const_get(name)
    return value unless type.is_a?(Class) && type.respond_to?(:members)
    fields = {}
    type.members.each { |member| fields[member] = value[member.to_s] }
    type.new(**fields)
  rescue StandardError
    value
  end

  def self.plain_hash(value)
    plain = {}
    value.each { |key, item| plain[key.to_s] = plain(item) }
    plain
  end
end

class TacOwner
  attr_reader :route, :service, :id, :instance, :metadata, :queue

  def initialize(request, instance, metadata)
    @route = request["route"]
    @service = request["service"]
    @id = request["owner"]
    @instance = instance
    @metadata = metadata
    @queue = []
    @disposed = false
  end

  def disposed?
    @disposed
  end

  def dispose
    return if @disposed
    @disposed = true
    @queue.clear
    TacServices.names(@metadata, "stops").each { |name| @instance.public_send(name) }
  end

  def matches?(request)
    @route == request["route"] && @service == request["service"]
  end
end

class TacServices
  MAX_OWNERS = 256
  MAX_QUEUED_EVENTS = 128
  MAX_REQUEST_BYTES = 1_048_576
  MAX_MEMBER_REQUEST_BYTES = 65_536
  MAX_EVENT_BYTES = 65_536
  HEXADECIMAL = "0123456789abcdefABCDEF"
  HYPHENS = [8, 13, 18, 23]

  @services = {}
  @owners = {}
  @executing = nil
  @handling = false

  def self.register(name, service_class, metadata_json)
    @services[name] = [service_class, JSON.parse(metadata_json)]
  end

  def self.names(metadata, key)
    metadata[key] || []
  end

  def self.ruby_name(metadata, name)
    (metadata["names"] || {})[name] || name
  end

  def self.invoke(text)
    return JSON.generate(TacAnswer.failure("Native owner is busy.")) if @handling
    @handling = true
    begin
      answer = handle(text)
      drain(@executing) if @executing
      JSON.generate(answer)
    rescue StandardError => error
      JSON.generate(TacAnswer.failure(error.message))
    ensure
      @executing = nil
      @handling = false
    end
  end

  def self.handle(text)
    request = read(text)
    return TacAnswer.failure("Invalid or oversized companion request.") if request.nil?
    operation = request["op"]
    return TacAnswer.failure("Companion request has no operation.") unless operation.is_a?(String) && !operation.empty?
    if operation == "host-dispose"
      dispose_all
      return TacAnswer.value(nil)
    end
    problem = identity_problem(request)
    return problem if problem
    handle_owned(request, operation)
  end

  def self.read(text)
    return nil if text.bytesize > MAX_REQUEST_BYTES
    request = JSON.parse(text)
    return nil unless request.is_a?(Hash)
    return nil if request["op"] != "init" && text.bytesize > MAX_MEMBER_REQUEST_BYTES
    request
  rescue StandardError
    nil
  end

  def self.identity_problem(request)
    return { "error" => "Missing companion route.", "code" => "TY_NATIVE_ROUTE" } unless present?(request["route"])
    return { "error" => "Missing companion name.", "code" => "TY_NATIVE_SERVICE" } unless present?(request["service"])
    return { "error" => "Missing native owner identity.", "code" => "TY_NATIVE_OWNER" } unless owner_id?(request["owner"])
    nil
  end

  def self.present?(value)
    value.is_a?(String) && !value.empty?
  end

  def self.owner_id?(text)
    return false unless text.is_a?(String) && text.size == 36
    text.chars.each_with_index do |character, index|
      if HYPHENS.include?(index)
        return false unless character == "-"
      else
        return false unless HEXADECIMAL.include?(character)
      end
    end
    true
  end

  def self.handle_owned(request, operation)
    if operation == "init"
      failed = activate(request)
      return failed if failed
    end
    owner = @owners[request["owner"]]
    return { "error" => "Stale native owner.", "code" => "TY_NATIVE_OWNER" } unless owner && owner.matches?(request)
    if operation == "dispose"
      dispose_owner(owner.id)
      return TacAnswer.value(nil)
    end
    @executing = owner
    return TacAnswer.value(describe(owner.metadata)) if operation == "init"
    name = request["name"]
    return TacAnswer.failure("Companion request has no member name.") unless present?(name)
    return receive(owner, name, request["value"]) if operation == "event"
    use_member(owner, operation, name, request)
  end

  def self.activate(request)
    existing = @owners[request["owner"]]
    return nil if existing && existing.matches?(request)
    dispose_owner(request["owner"]) if existing
    registered = @services[request["service"]]
    return { "error" => "Unknown native service.", "code" => "TY_NATIVE_SERVICE" } unless registered
    return TacAnswer.failure("Too many native activations.") if @owners.size >= MAX_OWNERS
    service_class, metadata = registered
    owner = TacOwner.new(request, service_class.new, metadata)
    @owners[owner.id] = owner
    @executing = owner
    assign_properties(owner, request["props"])
    start(owner)
  end

  def self.start(owner)
    names(owner.metadata, "starts").each { |name| owner.instance.public_send(name) }
    drain(owner)
    nil
  rescue StandardError
    dispose_owner(owner.id)
    TacAnswer.failure("Native initializer failed.")
  end

  def self.assign_properties(owner, properties)
    return unless properties.is_a?(Hash)
    required = names(owner.metadata, "required")
    fields = names(owner.metadata, "fields")
    properties.each do |key, value|
      field = ruby_name(owner.metadata, key)
      next unless required.include?(key) || required.include?(field) || required.include?("*")
      next unless fields.include?(key)
      owner.instance.instance_variable_set("@#{field}", value)
    end
  end

  def self.describe(metadata)
    listeners = names(metadata, "listeners").map do |listener|
      {
        "key" => listener["method"],
        "events" => listener["events"],
        "replay" => listener["replay"],
        "order" => listener["order"],
        "asynchronous" => listener["asynchronous"],
        "condition" => listener["condition"],
        "models" => listener["models"]
      }
    end
    { "fields" => names(metadata, "fields"), "methods" => names(metadata, "methods"), "listeners" => listeners }
  end

  def self.receive(owner, name, value)
    listener = names(owner.metadata, "listeners").find { |candidate| candidate["method"] == name }
    return TacAnswer.failure("Unknown companion listener: #{name}") unless listener
    method = owner.instance.method(name)
    payload = TacValues.model(listener["events"].first, value)
    result = method.arity == 0 ? owner.instance.public_send(name) : owner.instance.public_send(name, payload)
    republish(result, listener["returns"]) if result && !listener["asynchronous"]
    TacAnswer.value(nil)
  rescue StandardError
    TacAnswer.failure("Native event listener failed.")
  end

  def self.use_member(owner, operation, name, request)
    fields = names(owner.metadata, "fields")
    return TacAnswer.value(owner.instance.public_send(ruby_name(owner.metadata, name))) if operation == "get" && fields.include?(name)
    return write(owner, name, request["value"]) if operation == "set" && fields.include?(name)
    return call(owner, name, request["args"]) if operation == "call" && names(owner.metadata, "methods").include?(name)
    return TacAnswer.failure("Unknown companion member: #{name}") unless fields.include?(name) || names(owner.metadata, "methods").include?(name)
    TacAnswer.failure("Companion member does not support #{operation}: #{name}")
  end

  def self.write(owner, name, value)
    return TacAnswer.failure("Companion field is read-only: #{name}") unless names(owner.metadata, "writable").include?(name)
    owner.instance.public_send("#{ruby_name(owner.metadata, name)}=", value)
    TacAnswer.value(nil)
  end

  def self.call(owner, name, arguments)
    arguments = [] unless arguments.is_a?(Array)
    TacAnswer.value(owner.instance.public_send(ruby_name(owner.metadata, name), *arguments))
  end

  def self.publish(name, value, retain)
    owner = @executing
    return event_failure if owner.nil? || owner.disposed?
    encoded = JSON.generate(TacValues.plain(value))
    return event_failure if encoded.bytesize > MAX_EVENT_BYTES || owner.queue.size >= MAX_QUEUED_EVENTS
    owner.queue << [name.to_s, encoded, retain == true]
  rescue StandardError
    event_failure
  end

  def self.republish(result, returns)
    return unless returns.is_a?(String)
    items = result.is_a?(Array) ? result.first(32) : [result]
    items.each { |item| publish(returns, item, false) unless item.nil? }
  end

  def self.drain(owner)
    until owner.disposed? || owner.queue.empty?
      name, encoded, retain = owner.queue.shift
      send_event(owner, name, encoded, retain)
    end
  end

  def self.send_event(owner, name, encoded, retain)
    return unless TachyonWeb.respond_to?(:host_emit)
    payload = "{\"route\":#{JSON.generate(owner.route)},\"service\":#{JSON.generate(owner.service)},\"owner\":#{JSON.generate(owner.id)},\"name\":#{JSON.generate(name)},\"value\":#{encoded},\"retain\":#{retain}}"
    TachyonWeb.host_emit(payload)
  end

  def self.event_failure
    STDERR.write("Tachyon native event delivery failed.\n")
    nil
  end

  def self.dispose_owner(id)
    owner = @owners.delete(id)
    @executing = nil if @executing && @executing.id == id
    owner&.dispose
  rescue StandardError
    event_failure
  end

  def self.dispose_all
    active = @owners.values
    @owners = {}
    @executing = nil
    active.each(&:dispose)
  end
end

module TachyonWeb
  def self.invoke(text)
    TacServices.invoke(text)
  end
end
