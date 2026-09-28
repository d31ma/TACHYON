@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
  @Min(1)
  @Max(100)
  pageSize: integer = 20;
}
