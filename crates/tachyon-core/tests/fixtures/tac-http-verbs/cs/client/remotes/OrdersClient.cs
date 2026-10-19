[HTTPClient({"path":"/api","adapter":"orders","setup":"OrdersSetup"})]
public interface OrdersClient
{
    [PutRequest("/orders/{id}")]
    System.Threading.Tasks.Task<Models.SentRequest> replace([PathVariable("id")] string id, [RequestBody] BehaviorJSON order, [RequestHeader("X-Tenant")] string tenant);

    [PatchRequest("/orders/{id}")]
    System.Threading.Tasks.Task<Models.SentRequest> change([PathVariable("id")] string id, [RequestBody] BehaviorJSON change, [RequestHeader("X-Tenant")] string tenant);

    [DeleteRequest("/orders/{id}")]
    System.Threading.Tasks.Task<Models.SentRequest> remove([PathVariable("id")] string id, [RequestHeader("X-Reason")] string reason);
}
