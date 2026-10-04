#[Service]
class CatalogService
  # @return [Integer]
  attr_reader :count

  def initialize
    @products = ProductsClient.new
    @count = 0
  end

  # @return [void]
  def load
    @count = @products.list["items"].length
  end

  # @return [void]
  def add
    @products.save({ "name" => "anvil" })
  end
end
