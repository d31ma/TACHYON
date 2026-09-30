#[Service]
class CatalogService
  #[Needs]
  # @return [CatalogClient]
  attr_reader :catalog_client

  # @return [Integer]
  attr_reader :count

  def initialize
    @count = 0
  end

  # @return [void]
  def load
    @count = catalog_client.all["items"].length
  end
end
