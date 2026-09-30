#[Service]
class StockService
  # @return [Array<Product>]
  attr_reader :products
  # @return [Boolean]
  attr_reader :restocking

  def initialize
    @products = []
    @restocking = true
  end
end
