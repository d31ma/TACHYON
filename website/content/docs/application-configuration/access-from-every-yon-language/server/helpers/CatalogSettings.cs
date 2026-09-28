[Settings({"prefix":"YON_CATALOG_"})]
public sealed class CatalogSettings {
    [Min(1)]
    [Max(100)]
    public int pageSize = 20;
}
