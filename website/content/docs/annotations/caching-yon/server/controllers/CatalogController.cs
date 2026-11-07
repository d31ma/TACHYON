using System.Text.Json.Nodes;
using Models;

[RequestRoute("/catalog")]
[Public]
public sealed class CatalogController {
    [Needs]
    public CatalogService Catalog;

    [GetRoute]
    public CacheReport GetCatalog() => BuildReport().GetAwaiter().GetResult();

    private async System.Threading.Tasks.Task<CacheReport> BuildReport() {
        var sku = BehaviorJSON.From("anvil");
        var first = await Catalog.find(sku);
        var repeat = await Catalog.find(sku);
        await Catalog.write(sku, BehaviorJSON.From(99));
        var written = await Catalog.find(sku);
        await Catalog.invalidate(sku);
        var refreshed = await Catalog.find(sku);
        return new CacheReport(
            JsonNode.Parse(first.ToString()),
            JsonNode.Parse(repeat.ToString()),
            JsonNode.Parse(written.ToString()),
            JsonNode.Parse(refreshed.ToString()));
    }
}
