#[HTTPClient("/api")]
class CatalogClient
  # @param id [String]
  # @param view [String]
  # @param tenant [String]
  # @return [Hash]
  #[GetRequest("/products/{id}")]
  def find(
    #[PathVariable("id")]
    id,
    #[RequestParam("view")]
    view,
    #[RequestHeader("X-Tenant")]
    tenant
  )
  end

  # @param product [Hash]
  # @return [Hash]
  #[PostRequest("/products")]
  def save(
    #[RequestBody]
    product
  )
  end

  # @param id [Integer]
  # @param product [Hash]
  # @return [Hash]
  #[PutRequest("/products/{id}")]
  def replace(
    #[PathVariable("id")]
    id,
    #[RequestBody]
    product
  )
  end

  # @param id [Integer]
  # @param product [Hash]
  # @return [Hash]
  #[PatchRequest("/products/{id}")]
  def rename(
    #[PathVariable("id")]
    id,
    #[RequestBody]
    product
  )
  end

  # @param id [Integer]
  # @return [Hash]
  #[DeleteRequest("/products/{id}")]
  def remove(
    #[PathVariable("id")]
    id
  )
  end
end
