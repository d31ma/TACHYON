#[Setup]
class CatalogSetup
  def self.configure(execution)
    cache = MemoryCacheHelper.new
    { caches: { catalog: cache }, authorize: MemoryCacheHelper.method(:allow) }
  end
end
