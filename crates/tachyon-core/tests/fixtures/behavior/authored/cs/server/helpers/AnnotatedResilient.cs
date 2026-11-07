public sealed class AnnotatedResilient {
    [Needs]
    public BehaviorContext behaviors;
    public int attempts = 0;
    [Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})]
    public async System.Threading.Tasks.Task<BehaviorJSON> flaky() {
        await System.Threading.Tasks.Task.CompletedTask;
        attempts++;
        if (attempts < 3) throw new System.InvalidOperationException("transient");
        return BehaviorJSON.From(attempts);
    }
    [MaxAtOnce({"limit":2,"policy":"block"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> bounded(BehaviorJSON value) {
        await System.Threading.Tasks.Task.CompletedTask;
        return value;
    }
}

