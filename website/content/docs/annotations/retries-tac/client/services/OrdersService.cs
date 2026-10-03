[Service]
public sealed class OrdersService {
    public BehaviorContext behaviors;
    private const int FirstWorkingAttempt = 3;
    public int attempts = 0;

    [OnStart]
    public void open() {
        behaviors = Behaviors.create(typeof(OrdersSetup), "orders", new BehaviorSignal());
    }

    [Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})]
    public async System.Threading.Tasks.Task<BehaviorJSON> submit(BehaviorJSON id) {
        await System.Threading.Tasks.Task.CompletedTask;
        attempts++;
        FailFirstTwoAttempts();
        return ReportOf(id);
    }

    [MaxAtOnce({"limit":2,"policy":"block"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> report(BehaviorJSON id) {
        await System.Threading.Tasks.Task.CompletedTask;
        return ReportOf(id);
    }

    private BehaviorJSON ReportOf(BehaviorJSON id) {
        var fields = new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["id"] = id,
            ["attempts"] = BehaviorJSON.From(attempts),
        };
        return BehaviorJSON.Object(fields);
    }

    private void FailFirstTwoAttempts() {
        if (attempts < FirstWorkingAttempt) throw new System.InvalidOperationException("the checkout API is unavailable");
    }
}
