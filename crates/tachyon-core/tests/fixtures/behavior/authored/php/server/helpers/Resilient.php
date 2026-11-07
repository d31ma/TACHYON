<?php
class Resilient {
    #[Needs]
    public BehaviorContext $behaviors;
    public static int $attempts=0;
    #[Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})]
    public function flaky(): int {
        self::$attempts++;
        if (self::$attempts < 3) throw new Exception('transient');
        return self::$attempts;
    }
    #[MaxAtOnce({"limit":2,"policy":"reject"})]
    public function bounded(string $value): string {
        return $value;
    }
}

