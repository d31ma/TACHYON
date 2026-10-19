#[CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
#[Service]
class CatalogService
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
  def price(
    #[CacheKey]
    sku,
    note
  )
    Backend.prices += 1
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
