#[Service]
class InventoryService
  # @return [Array<Product>]
  attr_reader :products

  def initialize
    @products = [Product.new(name: "Anvil"), Product.new(name: "Rope")]
  end
end
