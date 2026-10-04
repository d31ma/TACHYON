// @Service
type CounterService struct {
	Count                     int64
	applicationEventPublisher ApplicationEventPublisher
}

func (counter *CounterService) Add() {
	counter.Count += 1
	counter.applicationEventPublisher.PublishEvent(CountChanged{Count: counter.Count}, false)
}
