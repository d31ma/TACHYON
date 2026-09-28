class Resilient {
    @Needs
    val behaviors: Behaviors.Context
    var attempts: Int = 0
    @Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})
    fun flaky(): Any? {
        attempts++
        if (attempts < 3) {
            throw IllegalStateException("transient")
        }
        return attempts
    }
    @MaxAtOnce({"limit":2,"policy":"block"})
    fun bounded(value: Any?): Any? { return value }
}

