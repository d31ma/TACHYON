#[TestSetup]
class OrdersTestSetup
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(execution)
    { http: { orders: { exchange: method(:exchange) } } }
  end

  # @param request [Hash]
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.exchange(request, execution)
    body = request["body"]
    {
      "method" => request["method"],
      "path" => request["path"],
      "header" => request["headers"].values.first,
      "item" => body.nil? ? "none" : body["item"]
    }
  end
end
