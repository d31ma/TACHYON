// @Service
type CounterService struct {
	Count int
}

func (counter *CounterService) Increment() int {
	counter.Count++
	return counter.Count
}
