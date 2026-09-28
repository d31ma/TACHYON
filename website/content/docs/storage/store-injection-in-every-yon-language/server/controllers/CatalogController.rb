#[RequestRoute("/catalog")]
#[Public]
class CatalogController
  #[Needs]
  # @return [CatalogStore]
  attr_reader :catalog_store

  # @return [Array<Hash>]
  #[GetRoute]
  def get_catalog
    catalog_store.find
  end
end
