// @Service
type StatusService struct {
	Online bool
	Scheme string
}

// @OnEvent({"replay":true})
func (status *StatusService) ConnectivityChanged(changed ConnectivityChanged) {
	status.Online = changed.Online
}

// @OnEvent({"replay":true})
func (status *StatusService) PreferencesChanged(changed PreferencesChanged) {
	status.Scheme = changed.Scheme
}
