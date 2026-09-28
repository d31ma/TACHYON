#[CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
#[HTTPClient({"adapter": "api"})]
class CatalogClient
  PAGE = "page"
  CALLS = "calls"

  attr_accessor :behaviors

  #[Cached]
  def read(
    #[PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})]
    page
  )
    Backend.calls += 1
    { CALLS => Backend.calls, PAGE => page }
  end

  #[Cached]
  #[GetRequest("/products")]
  def remote
    { "query" => { PAGE => 1 } }
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

  #[UpdateCache({"method":"read"})]
  def replace(value, page)
    raise "failed mutation" if value == "fail"
    { CALLS => value, PAGE => page }
  end

  #[ClearCache({"method":"read"})]
  def remove(page)
    true
  end
end
