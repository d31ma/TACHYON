@Service
class OrdersService {
    lateinit var behaviors: Behaviors.Context
    private var attempts: Int = 0

    @OnStart
    fun open() {
        val options = Behaviors.Options("orders", Behaviors.Signal())
        behaviors = Behaviors.create("OrdersSetup", options)
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
        if (attempts < FIRST_WORKING_ATTEMPT) throw IllegalStateException("the payment gateway is unavailable")
    }

    companion object {
        const val FIRST_WORKING_ATTEMPT = 3
    }
}
