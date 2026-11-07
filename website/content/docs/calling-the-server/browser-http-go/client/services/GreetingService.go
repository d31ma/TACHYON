// @Service
type GreetingService struct {
	// @Needs
	greetingClient *GreetingClient
	// @Needs
	overrideClient *OverrideClient
	Message        string
	Custom         string
	Loads          int
	Clears         int
}

func (service *GreetingService) Ask() error {
	service.Loads += 1
	greeting, err := service.greetingClient.Read()
	service.Message = greeting.Message
	return err
}

func (service *GreetingService) Clear() error {
	_, err := service.greetingClient.Clear()
	service.Clears += 1
	return err
}

func (service *GreetingService) AskCustom() error {
	greeting, err := service.overrideClient.Read()
	service.Custom = greeting.Message
	return err
}
