#[TestSetup]
class HelloTestSetup
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(execution)
    { http: { greeting: { exchange: method(:exchange) } } }
  end

  # @param request [Hash]
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.exchange(request, execution)
    { "message" => "Hi" }
  end
end
