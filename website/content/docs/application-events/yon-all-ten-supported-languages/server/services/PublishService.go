// @Service
type PublishService struct {
	// @Needs
	applicationEventPublisher *ApplicationEventPublisher
}

// @OnStart
func (service *PublishService) Initialize() {
	service.applicationEventPublisher.PublishEvent(ProbeReady{Value: 7}, false)
}
