// @Service
type LiveTotalService struct {
	Seen int64
}

// @OnEvent({"replay":true})
func (live *LiveTotalService) Changed(changed CartTotal) {
	live.Seen = changed.Total
}
