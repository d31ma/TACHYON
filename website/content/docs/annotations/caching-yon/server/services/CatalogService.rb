#[Service]
#[CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
class CatalogService
  attr_reader :reads

  #[OnStart]
  def open
    @reads = 0
    @behaviors = Behaviors.create(CatalogSetup, partition: "public-catalog", signal: BehaviorSignal.new)
  end

  #[Cached]
  def find(sku)
    @reads += 1
    entry(sku, @reads)
  end

  #[UpdateCache({"method":"find"})]
  def write(sku, reads)
    entry(sku, reads)
  end

  #[ClearCache({"method":"find"})]
  def invalidate(sku)
    entry(sku, @reads)
  end

  private

  def entry(sku, reads)
    { "sku" => sku, "reads" => reads }
  end
end
