#[RequestRoute("/catalog")]
#[Public]
class CatalogController
  #[Needs]
  # @return [ProductConverter]
  attr_reader :product_converter
  # @return [Summary]
  #[GetRoute]
  def get_catalog
    detail = Detail.new(name: "detail", secret: "detail-private")
    product_converter.map(Product.new(id: 7, name: "Anvil", detail: detail, items: [detail], secret: "product-private"))
  end
end
