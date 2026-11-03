@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
  @Min(1)
  @Max(100)
  final int pageSize;

  const CatalogSettings({this.pageSize = 20});
}
