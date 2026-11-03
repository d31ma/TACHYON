// @Service
type CounterService struct {
	Count int64
	// @SessionState
	Visits int64
	// @LocalState
	Theme string
}

// @OnStart
func (counter *CounterService) Start() {
	counter.Theme = counter.startingTheme()
}

func (counter *CounterService) Increment() {
	counter.Count++
}

func (counter *CounterService) startingTheme() string {
	if counter.Theme == "" {
		return "light"
	}
	return counter.Theme
}
