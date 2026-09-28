#[Service]
class CatalogService
  # @return [String]
  attr_reader :title

  def initialize
    @title = "Catalog"
  end

  # @param value [Integer]
  # @return [Integer]
  def double(value)
    value * 2
  end
end
