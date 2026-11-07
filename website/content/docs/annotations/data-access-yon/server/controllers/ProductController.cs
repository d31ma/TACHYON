using System.Text.Json.Nodes;
using Models;

[RequestRoute("/products")]
[Public]
sealed class ProductController {
    [Needs]
    ProductStore _products;

    [GetRoute]
    public ProductReport GetProduct() {
        var activeStatus = BehaviorJSON.From("active");
        var active = _products.findByStatus(activeStatus).GetAwaiter().GetResult();
        var activeCount = _products.countByStatus(activeStatus).GetAwaiter().GetResult();
        var page = BehaviorJSON.Parse("{\"size\":5}");
        var others = _products.excluding(activeStatus, page).GetAwaiter().GetResult();
        var first = _products.findById(1).GetAwaiter().GetResult();
        return new ProductReport(
            JsonNode.Parse(active.ToString()),
            JsonNode.Parse(activeCount.ToString()),
            JsonNode.Parse(others.ToString()),
            JsonNode.Parse(first.ToString()));
    }
}
