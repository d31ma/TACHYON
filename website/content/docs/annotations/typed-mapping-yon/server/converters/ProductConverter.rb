#[Converter]
class ProductConverter
  # @param product [Product]
  # @return [Summary]
  #[Convert({"fields":[{"target":"kind","constant":"product"}]})]
  def map(product)
  end
end
