// @Service
type CounterService struct {
	Count int64
}

func (counter *CounterService) Increment() {
	counter.Count += 1
}
