using System.Text.Json.Nodes;
using Models;

[RequestRoute("/catalog")]
[Public]
sealed class CatalogController {
    [GetRoute]
    public CatalogSummary GetCatalog() {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CatalogSetup), "public-catalog", signal);
        var mapper = new ProductConverter(context);
        var product = BehaviorJSON.Parse("{\"detail\":{\"id\":7},\"name\":\"Anvil\",\"extra\":\"dropped\"}");
        var summary = mapper.toSummary(product);
        return new CatalogSummary(JsonNode.Parse(summary.ToString()));
    }
}
