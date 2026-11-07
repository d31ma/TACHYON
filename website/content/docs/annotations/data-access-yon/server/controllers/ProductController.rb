#[RequestRoute("/products")]
#[Public]
class ProductController
  ACTIVE_STATUS = "active"

  #[Needs]
  # @return [ProductStore]
  attr_reader :product_store

  # @return [ProductReport]
  #[GetRoute]
  def get_product
    ProductReport.new(
      active: product_store.find_by_status(ACTIVE_STATUS),
      count: product_store.count_by_status(ACTIVE_STATUS),
      others: product_store.excluding(ACTIVE_STATUS, { "size" => 5 }),
      first: product_store.find_by_id(1)
    )
  end
end
