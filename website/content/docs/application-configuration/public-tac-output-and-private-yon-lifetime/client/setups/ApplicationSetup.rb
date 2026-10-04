#[Setup]
class ApplicationSetup
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(execution)
    { http: { publicConfiguration: { exchange: method(:exchange) } } }
  end

  # @param request [Hash]
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.exchange(request, execution)
    { "tac" => { "api" => { "url" => "/api" } } }
  end
end
