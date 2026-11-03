// @Service
type TallyService struct {
	// @Needs
	counter *CounterService
	Total   int
}

func (tally *TallyService) Bump() {
	tally.Total = tally.counter.Increment()
}
