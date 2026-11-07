class Resilient {
    final Behaviors.Context behaviors
    int attempts = 0
    Resilient(Behaviors.Context behaviors) { this.behaviors = behaviors }
    @Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})
    Object flaky() {
        attempts++
        if (attempts < 3) {
            throw new IllegalStateException("transient")
        }
        return attempts
    }
    @MaxAtOnce({"limit":2,"policy":"block"})
    Object bounded(Object value) { return value }
}

