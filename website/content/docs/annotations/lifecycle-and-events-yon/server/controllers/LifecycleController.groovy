import models.Lifecycle

@RequestRoute("/lifecycle")
@Public
class LifecycleController {
  @Needs
  final AuditService audit
  @Needs
  final CheckoutService checkout
  @GetRoute
  Lifecycle getLifecycle() {
    return new Lifecycle(audit.log, audit.settled)
  }
}
