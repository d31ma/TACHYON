#[Settings({"prefix":"YON_CATALOG_"})]
class CatalogSettings
  # @return [Integer]
  #[Min(1)]
  #[Max(100)]
  attr_reader :page_size
end
