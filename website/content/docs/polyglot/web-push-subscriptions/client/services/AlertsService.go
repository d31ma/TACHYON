// @Service
type AlertsService struct {
	Endpoint string
	Problem  string
	// @Needs
	push Push
}

func (service *AlertsService) TurnOn() {
	service.keepProblem(service.push.Subscribe(service.serverKey()))
}

func (service *AlertsService) TurnOff() {
	service.keepProblem(service.push.Unsubscribe())
}

// @OnEvent
func (service *AlertsService) Subscribed(subscribed PushSubscribed) {
	service.Endpoint = subscribed.Endpoint
}

// @OnEvent
func (service *AlertsService) Unsubscribed(unsubscribed PushUnsubscribed) {
	service.Endpoint = ""
}

// @OnEvent
func (service *AlertsService) Failed(failed BrowserCallFailed) {
	service.Problem = failed.Message
}

func (service *AlertsService) serverKey() string {
	return "BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U"
}

func (service *AlertsService) keepProblem(problem error) {
	if problem != nil {
		service.Problem = problem.Error()
	}
}
