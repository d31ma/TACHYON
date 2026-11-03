#[RequestRoute("/products")]
#[Public]
class ProductsController
  # @return [Array<Product>]
  #[GetRoute]
  def get_products
    [Product.new(name: "anvil")]
  end

  # @param product [Product]
  # @return [Product]
  #[PostRoute]
  #[ResponseStatus(201)]
  def post_product(
    #[FromBody]
    product
  )
    product
  end
end
