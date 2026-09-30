#[Service]
class CatalogService
  # @return [Integer]
  attr_reader :count

  def initialize
    @catalog = CatalogClient.new
    @count = 0
  end

  # @return [void]
  def load
    @count = @catalog.all["items"].length
  end
end
