// @Service
type TickerService struct {
	Seconds int
}

// @Every("1s")
func (ticker *TickerService) Count() {
	ticker.Seconds++
}
