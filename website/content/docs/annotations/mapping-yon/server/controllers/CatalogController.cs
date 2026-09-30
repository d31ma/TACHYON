using System.Text.Json.Nodes;
using Models;

[RequestRoute("/catalog")]
[Public]
sealed class CatalogController {
    [Needs]
    ProductConverter _converter;

    [GetRoute]
    public CatalogSummary GetCatalog() {
        var product = BehaviorJSON.Parse("{\"detail\":{\"id\":7},\"name\":\"Anvil\",\"extra\":\"dropped\"}");
        var summary = _converter.toSummary(product);
        return new CatalogSummary(JsonNode.Parse(summary.ToString()));
    }
}
