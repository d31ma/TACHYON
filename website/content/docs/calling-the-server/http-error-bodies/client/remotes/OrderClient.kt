import models.OrderInput

@HTTPClient("/api")
interface OrderClient {
    @PostRequest("/orders")
    suspend fun send(@RequestBody order: Map<String, Any>): OrderInput
}
