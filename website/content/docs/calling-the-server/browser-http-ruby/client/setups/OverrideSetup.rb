#[Setup]
class OverrideSetup
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(execution)
    { http: { override: { exchange: method(:exchange) } } }
  end

  # @param request [Hash]
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.exchange(request, execution)
    { "message" => "Custom adapter wins" }
  end
end
