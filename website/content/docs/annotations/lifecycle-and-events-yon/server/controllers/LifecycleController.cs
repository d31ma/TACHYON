using Models;

[RequestRoute("/lifecycle")]
[Public]
class LifecycleController {
  [Needs]
  readonly AuditService _audit;
  [Needs]
  readonly CheckoutService _checkout;
  [GetRoute]
  public Lifecycle GetLifecycle() {
    return new Lifecycle(_audit.Log, _audit.Settled);
  }
}
