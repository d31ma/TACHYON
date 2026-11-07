#[RequestRoute("/catalog")]
#[Public]
class CatalogController
  #[Needs]
  # @return [CatalogService]
  attr_reader :catalog_service

  # @return [CacheReport]
  #[GetRoute]
  def get_catalog
    sku = "anvil"
    first = catalog_service.find(sku)
    repeat = catalog_service.find(sku)
    catalog_service.write(sku, 99)
    written = catalog_service.find(sku)
    catalog_service.invalidate(sku)
    refreshed = catalog_service.find(sku)
    CacheReport.new(first: first, repeat: repeat, written: written, refreshed: refreshed)
  end
end
