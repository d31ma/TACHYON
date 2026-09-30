#[RequestRoute("/catalog")]
#[Public]
class CatalogController
  #[Needs]
  # @return [ProductConverter]
  attr_reader :product_converter

  # @return [CatalogSummary]
  #[GetRoute]
  def get_catalog
    product = { "detail" => { "id" => 7 }, "name" => "Anvil", "extra" => "dropped" }
    CatalogSummary.new(summary: product_converter.to_summary(product))
  end
end
