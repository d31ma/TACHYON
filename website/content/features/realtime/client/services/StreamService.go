// @Service
type StreamService struct {
	// @Needs
	ticksClient *TicksClient
	Ticks       []Tick
}

// @OnEvent
func (service *StreamService) AddTick(tick Tick) {
	service.Ticks = service.recent(append(service.Ticks, tick))
}

func (StreamService) tickLimit() int { return 128 }

func (service *StreamService) recent(ticks []Tick) []Tick {
	if len(ticks) > service.tickLimit() {
		return ticks[len(ticks)-service.tickLimit():]
	}
	return ticks
}
