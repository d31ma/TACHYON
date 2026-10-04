#[Service]
class CatalogService
  #[Needs]
  # @return [ProductConverter]
  attr_reader :product_converter
  # @return [String]
  attr_reader :result

  def initialize
    @result = ""
  end

  # @return [void]
  def load
    detail = Detail.new(name: "detail", secret: "detail-private")
    product = product_converter.copy(Product.new(id: 7, name: "Anvil", detail: detail, items: [detail], secret: "product-private"))
    @result = text(product_converter.map(product))
  end

  private

  # @param summary [Summary]
  # @return [String]
  def text(summary)
    "#{summary.kind} #{summary.id}: #{summary.name}, #{summary.detail.name}, #{summary.items.length}"
  end
end
