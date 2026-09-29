@Settings({"prefix":"YON_CATALOG_"})
export default class CatalogSettings {
  @Min(1)
  @Max(100)
  pageSize: integer = 20;
}
