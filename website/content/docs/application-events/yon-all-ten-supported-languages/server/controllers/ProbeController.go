// @RequestRoute("/probe")
// @Public
type ProbeController struct {
	// @Needs
	probe *ProbeService
	// @Needs
	publish *PublishService
}

// @GetRoute
func (controller *ProbeController) GetProbe() *ProbeReady {
	return controller.probe.Last
}
