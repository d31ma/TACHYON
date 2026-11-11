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
    raise conflict if request["path"] == "/api/claims/a-7"
    body = request["body"]
    {
      "method" => request["method"],
      "path" => request["path"],
      "header" => request["headers"].values.first,
      "item" => body.nil? ? "none" : body["item"]
    }
  end

  # @return [BehaviorError]
  def self.conflict
    BehaviorError.new("HTTP_STATUS", "private fake text", nil, 409, { "reason" => "taken", "holder" => "south" })
  end
end
