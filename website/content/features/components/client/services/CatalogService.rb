#[Service]
class CatalogService
  # @return [Array<Product>]
  attr_reader :products

  def initialize
    @products = [
      Product.new(id: "anvil", name: "Anvil", price: "£42.00"),
      Product.new(id: "rope", name: "Rope", price: "£8.00"),
    ]
  end
end
