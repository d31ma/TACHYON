[Test(typeof(OrdersTestSetup), {"partition":"tac-http-verbs"})]
public sealed class OrderServiceTest
{
    [Fake]
    public OrdersClient _ordersClient = null;

    public async System.Threading.Tasks.Task testPutSendsThePathTheHeaderAndTheBody()
    {
        var sent = await construct<OrderService>().replace();
        Assert.equals("PUT", sent.Method);
        Assert.equals("/api/orders/a-7", sent.Path);
        Assert.equals("north", sent.Header);
        Assert.equals("hammer", sent.Item);
    }

    public async System.Threading.Tasks.Task testPatchSendsThePathTheHeaderAndTheBody()
    {
        var sent = await construct<OrderService>().change();
        Assert.equals("PATCH", sent.Method);
        Assert.equals("/api/orders/a-7", sent.Path);
        Assert.equals("south", sent.Header);
        Assert.equals("nails", sent.Item);
    }

    public async System.Threading.Tasks.Task testDeleteSendsThePathAndTheHeaderWithoutABody()
    {
        var sent = await construct<OrderService>().remove();
        Assert.equals("DELETE", sent.Method);
        Assert.equals("/api/orders/a-7", sent.Path);
        Assert.equals("duplicate", sent.Header);
        Assert.equals("none", sent.Item);
    }
}
