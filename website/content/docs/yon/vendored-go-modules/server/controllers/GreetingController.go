import "example.com/greet"

// @RequestRoute("/api/greeting")
// @Public
type GreetingController struct {
}

// @GetRoute
func (controller *GreetingController) GetGreeting() map[string]string {
	return map[string]string{"text": greet.Hello("shop")}
}
