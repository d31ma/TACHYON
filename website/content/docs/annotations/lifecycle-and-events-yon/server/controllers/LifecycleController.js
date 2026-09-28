/** @import { Lifecycle } from '/models/Lifecycle.ts' */
@RequestRoute("/lifecycle")
@Public
class LifecycleController {
  /** @type {AuditService} */
  @Needs
  auditService;
  /** @type {CheckoutService} */
  @Needs
  checkoutService;
  /** @returns {Lifecycle} */
  @GetRoute
  getLifecycle() {
    return {
      log: this.auditService.log,
      settled: this.auditService.settled,
      audited: this.auditService.audited,
    };
  }
}
