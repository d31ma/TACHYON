class Resilient
  attr_accessor :behaviors

  @attempts = 0

  class << self
    attr_accessor :attempts
  end

  #[Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})]
  def flaky
    Resilient.attempts += 1
    raise "transient" if Resilient.attempts < 3
    Resilient.attempts
  end

  #[MaxAtOnce({"limit":2,"policy":"reject"})]
  def bounded(value)
    value
  end
end
