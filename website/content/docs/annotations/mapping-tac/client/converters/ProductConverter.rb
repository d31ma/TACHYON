#[Converter]
class ProductConverter
  # @param product [Hash]
  # @return [Hash]
  #[Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})]
  def to_summary(product)
  end
end
