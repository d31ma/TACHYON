#[RequestRoute("/products")]
#[Public]
class ProductsController
  # @return [Array]
  #[GetRoute]
  def get_products
    [200, { "content-type" => "text/html; charset=utf-8" }, ["<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>"]]
  end
end
