import type { Lifecycle } from '/models/Lifecycle.ts'

@RequestRoute("/lifecycle")
@Public
class LifecycleController {
  @Needs
  readonly audit!: AuditService;
  @Needs
  readonly checkout!: CheckoutService;
  @GetRoute
  getLifecycle(): Lifecycle {
    return { log: this.audit.log, settled: this.audit.settled, audited: this.audit.audited };
  }
}
