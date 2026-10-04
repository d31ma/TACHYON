// @HTTPClient("/api")
type GreetingClient interface {
	// @GetRequest("/greeting")
	GetGreeting() (Greeting, error)
}
