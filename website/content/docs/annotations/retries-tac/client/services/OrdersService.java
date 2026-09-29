@Service
class OrdersService {
    Behaviors.Context behaviors;
    private static final int FIRST_WORKING_ATTEMPT = 3;
    private int attempts = 0;

    @OnStart
    void open() {
        behaviors = Behaviors.create("OrdersSetup", new Behaviors.Options("orders", new Behaviors.Signal()));
    }

    @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
    Object submit(String id) {
        attempts++;
        failFirstTwoAttempts();
        return reportOf(id);
    }

    @MaxAtOnce({"limit":2,"policy":"block"})
    Object report(String id) {
        return reportOf(id);
    }

    private Object reportOf(String id) {
        return java.util.Map.of("id", id, "attempts", attempts);
    }

    private void failFirstTwoAttempts() {
        if (attempts < FIRST_WORKING_ATTEMPT) {
            throw new IllegalStateException("the checkout API is unavailable");
        }
    }
}
