#[RequestRoute("/products")]
#[Public]
class ProductsController
  #[Needs]
  # @return [ProductService]
  attr_reader :product_service

  # @param product [Product]
  # @return [Product]
  #[PostRoute]
  def post_product(
    #[FromBody]
    product
  )
    product_service.accept({ "name" => product.name, "level" => product.level })
    product
  end
end
