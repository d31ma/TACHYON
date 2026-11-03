// @Service
type CounterService struct {
	Count int
}

func (counter *CounterService) Increment() {
	counter.Count += 1
}
