// @HTTPClient("/ticks")
type TicksClient interface {
	// @StreamRequest
	StreamTicks(tick Tick)
}
