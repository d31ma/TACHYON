#[RequestRoute("/catalog")]
#[Public]
class CatalogController
  #[Needs]
  # @return [CatalogService]
  attr_reader :catalog_service

  # @return [Array<Hash>]
  #[GetRoute]
  def get_catalog
    catalog_service.find
  end
end
