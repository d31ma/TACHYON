using System.Text.Json.Nodes;
using Models;

[RequestRoute("/products")]
[Public]
sealed class ProductController {
    [GetRoute]
    public ProductReport GetProduct() {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CatalogSetup), "public-catalog", signal);
        var products = new ProductStore(context);
        var activeStatus = BehaviorJSON.From("active");
        var active = products.findByStatus(activeStatus).GetAwaiter().GetResult();
        var activeCount = products.countByStatus(activeStatus).GetAwaiter().GetResult();
        var page = products.bounded(BehaviorJSON.Parse("{\"size\":5}"));
        var others = products.excluding(activeStatus, page).GetAwaiter().GetResult();
        return new ProductReport(
            JsonNode.Parse(active.ToString()),
            JsonNode.Parse(activeCount.ToString()),
            JsonNode.Parse(others.ToString()));
    }
}
