import models.Lifecycle

@RequestRoute("/lifecycle")
@Public
class LifecycleController {
  @Needs
  lateinit var audit: AuditService
  @Needs
  lateinit var checkout: CheckoutService
  @GetRoute
  fun getLifecycle(): Lifecycle {
    return Lifecycle(audit.log, audit.settled)
  }
}
