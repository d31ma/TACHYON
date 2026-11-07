#[RequestRoute("/lifecycle")]
#[Public]
class LifecycleController
  #[Needs]
  # @return [AuditService]
  attr_reader :audit_service

  #[Needs]
  # @return [CheckoutService]
  attr_reader :checkout_service

  # @return [Lifecycle]
  #[GetRoute]
  def get_lifecycle
    Lifecycle.new(log: audit_service.log, settled: audit_service.settled)
  end
end
