#[RequestRoute("/products")]
#[Public]
class ProductsController
  #[Needs]
  #[Named("ProductsService")]
  # @return [ProductsService]
  attr_reader :catalog

  # @return [Array<Product>]
  #[GetRoute]
  def get_products
    catalog.list
  end

  # @param id [Integer]
  # @return [Product]
  #[GetRoute("/{id}")]
  def get_product(
    #[FromPath]
    id
  )
    catalog.find(id)
  end
end
