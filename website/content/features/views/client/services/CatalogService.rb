#[Service]
class CatalogService
  # @return [String]
  attr_reader :title

  def initialize
    @title = "Catalog"
  end
end
