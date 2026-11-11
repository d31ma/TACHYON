[HTTPClient({"path":"/api","adapter":"orders","setup":"OrderSetup"})]
public interface OrderClient
{
    [PostRequest("/orders")]
    System.Threading.Tasks.Task<Models.OrderInput> send([RequestBody] BehaviorJSON order);
}
