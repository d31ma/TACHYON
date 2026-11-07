#[Service]
class ProductService
  # @param input [Hash]
  # @return [Hash]
  def accept(
    #[Check(ProductInput)]
    input
  )
    input
  end
end
