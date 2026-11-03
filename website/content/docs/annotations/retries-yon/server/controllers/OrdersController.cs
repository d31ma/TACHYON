using System.Text.Json.Nodes;
using Models;

[RequestRoute("/orders")]
[Public]
public sealed class OrdersController {
    [Needs]
    public OrdersService Orders;

    [GetRoute]
    public OrderReport GetOrders() => BuildReport().GetAwaiter().GetResult();

    private async System.Threading.Tasks.Task<OrderReport> BuildReport() {
        var id = BehaviorJSON.From("A-1");
        var submitted = await Orders.submit(id);
        var report = await Orders.report(id);
        return new OrderReport(
            JsonNode.Parse(submitted.ToString()),
            JsonNode.Parse(report.ToString()));
    }
}
