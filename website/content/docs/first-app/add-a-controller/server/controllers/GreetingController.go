// @RequestRoute("/api/greeting")
// @Public
type GreetingController struct{}

// @GetRoute
func (controller *GreetingController) GetGreeting() Greeting {
	return Greeting{Message: "Hello from Yon"}
}
