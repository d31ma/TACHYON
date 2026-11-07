#[Service]
class CatalogService
  #[Needs]
  # @return [ProductConverter]
  attr_reader :product_converter
  # @return [String]
  attr_reader :title

  def initialize
    @title = "Not mapped."
  end

  # @return [void]
  def load
    product = { "detail" => { "id" => 7 }, "name" => "Anvil", "extra" => "dropped" }
    @title = title_of(product_converter.to_summary(product))
  end

  private

  # @param summary [Hash]
  # @return [String]
  def title_of(summary)
    "#{summary["kind"]} #{summary["productId"]}: #{summary["title"]}"
  end
end
