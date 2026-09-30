#[RequestRoute("/catalog")]
#[Public]
class CatalogController
  #[Needs]
  # @return [CatalogService]
  attr_reader :catalog_service

  # @return [Hash]
  #[GetRoute]
  def get_catalog
    catalog_service.products
  end
end
