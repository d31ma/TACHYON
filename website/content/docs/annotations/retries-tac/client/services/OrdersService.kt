@Service
class OrdersService {
    lateinit var behaviors: Behaviors.Context
    var attempts: Int = 0

    @OnStart
    fun open() {
        behaviors = Behaviors.create("OrdersSetup", Behaviors.Options("orders", Behaviors.Signal()))
    }

    @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
    fun submit(id: String): Any? {
        attempts++
        failFirstTwoAttempts()
        return reportOf(id)
    }

    @MaxAtOnce({"limit":2,"policy":"block"})
    fun report(id: String): Any? { return reportOf(id) }

    private fun reportOf(id: String): Any? = mapOf("id" to id, "attempts" to attempts)

    private fun failFirstTwoAttempts() {
        if (attempts < FIRST_WORKING_ATTEMPT) throw IllegalStateException("the checkout API is unavailable")
    }

    companion object {
        const val FIRST_WORKING_ATTEMPT = 3
    }
}
