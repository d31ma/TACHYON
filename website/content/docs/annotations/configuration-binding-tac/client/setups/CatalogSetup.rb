#[Setup]
#[Settings({"prefix":"TAC_CATALOG_"})]
#[WhenProfile("public")]
#[WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]
class CatalogSetup
  # @return [Integer]
  #[Min(1)]
  #[Max(100)]
  attr_reader :page_size

  # @return [Boolean]
  attr_reader :cache_enabled

  # @return [String]
  attr_reader :title

  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(execution)
    {}
  end
end
