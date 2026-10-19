import models.SentRequest

@HTTPClient({"path":"/api","adapter":"orders"})
interface OrdersClient {
    @PutRequest("/orders/{id}")
    suspend fun replace(@PathVariable("id") id: String, @RequestBody order: Map<String, String>, @RequestHeader("X-Tenant") tenant: String): SentRequest

    @PatchRequest("/orders/{id}")
    suspend fun change(@PathVariable("id") id: String, @RequestBody change: Map<String, String>, @RequestHeader("X-Tenant") tenant: String): SentRequest

    @DeleteRequest("/orders/{id}")
    suspend fun remove(@PathVariable("id") id: String, @RequestHeader("X-Reason") reason: String): SentRequest
}
