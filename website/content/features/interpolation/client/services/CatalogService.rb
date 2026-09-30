#[Service]
class CatalogService
  # @return [String]
  attr_reader :title
  # @return [Integer]
  attr_reader :count
  # @return [Product]
  attr_reader :product

  def initialize
    @title = "Catalog"
    @count = 6
    @product = Product.new(name: "Anvil", price: "£42.00")
  end
end
