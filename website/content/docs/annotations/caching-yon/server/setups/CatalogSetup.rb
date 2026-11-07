#[Setup]
class CatalogSetup
  # @param execution [Object]
  # @return [Hash]
  def self.configure(execution)
    cache = MemoryCacheDelegate.new
    { caches: { catalog: cache }, authorize: MemoryCacheDelegate.method(:allow) }
  end
end
