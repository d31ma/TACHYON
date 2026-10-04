// @Service
type CounterService struct {
	// @Needs
	greetingClient *GreetingClient
	Count          int
	Message        string
}

func (counter *CounterService) Increment() {
	counter.Count += 1
}

func (counter *CounterService) AskServer() error {
	greeting, err := counter.greetingClient.GetGreeting()
	counter.Message = greeting.Message
	return err
}
