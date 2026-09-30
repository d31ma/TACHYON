#[HTTPClient]
class CatalogClient
  # @param page [Integer]
  # @return [Hash]
  #[GetRequest("/products")]
  def all(
    #[RequestParam("page")]
    page
  )
  end

  # @param product [Hash]
  # @return [Hash]
  #[PostRequest("/products")]
  def add(
    #[RequestBody]
    product
  )
  end
end
