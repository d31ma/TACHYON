// @Service
type ProbeService struct {
	Last *ProbeReady
}

// @OnEvent
func (probe *ProbeService) Observe(ready ProbeReady) {
	probe.Last = &ready
}

// @OnStop
func (probe *ProbeService) Destroy() {
	probe.Last = nil
}
