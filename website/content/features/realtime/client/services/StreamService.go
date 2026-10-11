// @Service
type StreamService struct {
	// @Needs
	ticksClient *TicksClient
	Ticks       []Tick
}

// @OnEvent
func (service *StreamService) AddTick(tick Tick) {
	service.Ticks = append(service.Ticks, tick)
	if len(service.Ticks) > 128 {
		service.Ticks = service.Ticks[len(service.Ticks)-128:]
	}
}
