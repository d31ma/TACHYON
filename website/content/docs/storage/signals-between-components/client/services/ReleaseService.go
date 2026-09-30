// @Service
type ReleaseService struct {
	Release                   string
	applicationEventPublisher ApplicationEventPublisher
}

func (release *ReleaseService) Announce() {
	changed := ReleaseChanged{Release: "TACHYON"}
	release.applicationEventPublisher.PublishEvent(changed, true)
}

// @OnEvent({"replay":true})
func (release *ReleaseService) Receive(changed ReleaseChanged) {
	release.Release = changed.Release
}
