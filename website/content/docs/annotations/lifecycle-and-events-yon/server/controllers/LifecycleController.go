// @RequestRoute("/lifecycle")
// @Public
type LifecycleController struct {
	// @Needs
	audit *AuditService
	// @Needs
	checkout *CheckoutService
}

// @GetRoute
func (controller *LifecycleController) GetLifecycle() Lifecycle {
	return Lifecycle{Log: controller.audit.log, Settled: controller.audit.settled}
}
