#[RequestRoute("/catalog")]
#[Public]
class CatalogController
  #[Needs]
  # @return [CatalogClient]
  attr_reader :catalog_client

  # @return [Hash]
  #[GetRoute]
  def get_catalog
    {
      "found" => catalog_client.find("anvil", "full", "acme"),
      "saved" => catalog_client.save({ "sku" => "anvil", "level" => 3 }),
      "replaced" => catalog_client.replace(5, { "sku" => "anvil", "level" => 4 }),
      "renamed" => catalog_client.rename(6, { "sku" => "hammer", "level" => 1 }),
      "removed" => catalog_client.remove(7)
    }
  end
end
