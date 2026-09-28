#[Service]
#[Lifetime("transient")]
class ProductsService
  # @return [Array<Product>]
  def list
    [find(1)]
  end

  # @param id [Integer]
  # @return [Product]
  def find(id)
    Product.new(id: id, name: "Anvil")
  end
end
