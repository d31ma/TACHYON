#[Service]
#[CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
class CatalogService
  # @return [Integer]
  attr_reader :reads

  # @return [void]
  #[OnStart]
  def open
    @reads = 0
    @behaviors = Behaviors.create(CatalogSetup, partition: "public-catalog", signal: BehaviorSignal.new)
  end

  # @param sku [String]
  # @return [Hash]
  #[Cached]
  def find(sku)
    @reads += 1
    entry(sku, @reads)
  end

  # @param sku [String]
  # @param reads [Integer]
  # @return [Hash]
  #[UpdateCache({"method":"find"})]
  def write(sku, reads)
    entry(sku, reads)
  end

  # @param sku [String]
  # @return [Hash]
  #[ClearCache({"method":"find"})]
  def invalidate(sku)
    entry(sku, @reads)
  end

  private

  # @param sku [String]
  # @param reads [Integer]
  # @return [Hash]
  def entry(sku, reads)
    { "sku" => sku, "reads" => reads }
  end
end
