import "sync/atomic"

type Resilient struct {
	// @Needs
	behaviors *BehaviorContext
	// @Needs
	attempts *atomic.Int64
}

// @Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})
func (resilient *Resilient) Flaky() (BehaviorValue, error) {
	count := resilient.attempts.Add(1)
	if count < 3 {
		return nil, &BehaviorError{Code: "adapter", Message: "transient"}
	}
	return count, nil
}

// @MaxAtOnce({"limit":2,"policy":"block"})
func (resilient *Resilient) Bounded(value BehaviorValue) (BehaviorValue, error) { return value, nil }
