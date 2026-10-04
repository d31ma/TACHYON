#[Converter]
class ProductConverter
  # @param product [Product]
  # @return [Summary]
  #[Convert({"fields":[{"target":"kind","constant":"product"}]})]
  def map(product)
  end

  # @param product [Product]
  # @return [Product]
  def copy(product)
  end
end
