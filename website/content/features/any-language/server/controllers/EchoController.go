// @RequestRoute("/echo")
// @Public
type EchoController struct {
}

// @PostRoute
func (controller *EchoController) PostEcho(
	// @FromBody
	message Message,
) Message {
	return message
}
