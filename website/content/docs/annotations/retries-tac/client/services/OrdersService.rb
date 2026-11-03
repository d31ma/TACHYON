#[Service]
class OrdersService
  FIRST_WORKING_ATTEMPT = 3

  # @return [Integer]
  attr_reader :attempts

  # @return [void]
  #[OnStart]
  def open
    @attempts = 0
    @behaviors = Behaviors.create(OrdersSetup, partition: "orders", signal: BehaviorSignal.new)
  end

  # @param id [String]
  # @return [Hash]
  #[Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})]
  def submit(id)
    @attempts += 1
    fail_first_two_attempts
    report_of(id)
  end

  # @param id [String]
  # @return [Hash]
  #[MaxAtOnce({"limit":2,"policy":"reject"})]
  def report(id)
    report_of(id)
  end

  private

  # @param id [String]
  # @return [Hash]
  def report_of(id)
    { "id" => id, "attempts" => @attempts }
  end

  # @return [void]
  def fail_first_two_attempts
    raise "the checkout API is unavailable" if @attempts < FIRST_WORKING_ATTEMPT
  end
end
