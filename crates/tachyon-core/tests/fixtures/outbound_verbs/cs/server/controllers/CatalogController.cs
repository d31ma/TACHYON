[RequestRoute("/catalog")]
[Public]
class CatalogController {
    [Needs]
    readonly CatalogClient _catalog;

    [GetRoute]
    public object GetCatalog() {
        return new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["found"] = _catalog.Find("anvil", "full", "acme").GetAwaiter().GetResult(),
            ["saved"] = _catalog.Save(BehaviorJSON.Parse("{\"sku\":\"anvil\",\"level\":3}")).GetAwaiter().GetResult(),
            ["replaced"] = _catalog.Replace(5, BehaviorJSON.Parse("{\"sku\":\"anvil\",\"level\":4}")).GetAwaiter().GetResult(),
            ["renamed"] = _catalog.Rename(6, BehaviorJSON.Parse("{\"sku\":\"hammer\",\"level\":1}")).GetAwaiter().GetResult(),
            ["removed"] = _catalog.Remove(7).GetAwaiter().GetResult(),
        };
    }
}
