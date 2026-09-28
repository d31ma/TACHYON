// @RequestRoute("/status")
// @Public
type StatusController struct {
}

// @GetRoute
func (controller *StatusController) GetStatus() string {
	return "ok"
}
