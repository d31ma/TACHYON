#[Setup]
class CatalogSetup
  # @param execution [Object]
  # @return [Hash]
  def self.configure(execution)
    cache = MemoryCacheHelper.new
    { caches: { catalog: cache }, authorize: MemoryCacheHelper.method(:allow) }
  end
end
