[Service]
public sealed class OrderService
{
    [Needs]
    private readonly OrdersClient _ordersClient;

    public System.Threading.Tasks.Task<Models.SentRequest> replace()
    {
        return _ordersClient.replace("a-7", BehaviorJSON.Parse("{\"item\":\"hammer\"}"), "north");
    }

    public System.Threading.Tasks.Task<Models.SentRequest> change()
    {
        return _ordersClient.change("a-7", BehaviorJSON.Parse("{\"item\":\"nails\"}"), "south");
    }

    public System.Threading.Tasks.Task<Models.SentRequest> remove()
    {
        return _ordersClient.remove("a-7", "duplicate");
    }
}
