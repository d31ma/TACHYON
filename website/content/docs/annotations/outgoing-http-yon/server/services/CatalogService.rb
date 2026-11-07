#[Service]
class CatalogService
  #[Needs]
  # @return [CatalogClient]
  attr_reader :catalog_client

  # @return [Hash]
  def products
    catalog_client.all(1)
  end
end
