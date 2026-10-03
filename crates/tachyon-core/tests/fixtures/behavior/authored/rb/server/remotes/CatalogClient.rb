#[CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
#[HTTPClient({"adapter": "api"})]
class CatalogClient
  #[Cached]
  #[GetRequest("/products")]
  def remote
  end

  #[GetRequest("/products")]
  def search(
    #[RequestParam("page")]
    page
  )
  end

  #[GetRequest("/products/{id}")]
  def find(
    #[PathVariable]
    id
  )
  end
end
