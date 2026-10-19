import models.SentRequest

@Service
class OrderService {
    @Needs
    private val ordersClient: OrdersClient

    suspend fun replace(): SentRequest = ordersClient.replace("a-7", mapOf("item" to "hammer"), "north")

    suspend fun change(): SentRequest = ordersClient.change("a-7", mapOf("item" to "nails"), "south")

    suspend fun remove(): SentRequest = ordersClient.remove("a-7", "duplicate")
}
