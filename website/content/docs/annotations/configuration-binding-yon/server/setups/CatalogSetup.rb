#[Setup]
#[Settings({"prefix":"YON_CATALOG_"})]
#[WhenProfile("development")]
#[WhenSetting({"name":"YON_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]
class CatalogSetup
  # @return [Integer]
  #[Min(1)]
  #[Max(100)]
  attr_reader :page_size

  # @return [Boolean]
  attr_reader :cache_enabled

  # @param execution [Object]
  # @return [Hash]
  def self.configure(execution)
    {}
  end
end
