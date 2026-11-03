#[Setup]
class ProductsSetup
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(execution)
    { http: { products: { exchange: method(:exchange) } } }
  end

  # @param request [Hash]
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.exchange(request, execution)
    { "items" => ["anvil", "hammer"] }
  end
end
