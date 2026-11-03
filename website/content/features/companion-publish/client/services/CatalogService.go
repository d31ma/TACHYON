// @Service
type CatalogService struct {
	Count                     int64
	applicationEventPublisher ApplicationEventPublisher
}

// @OnStart
func (catalog *CatalogService) Initialize() {
	catalog.announce()
}

func (catalog *CatalogService) announce() {
	catalog.applicationEventPublisher.PublishEvent(CatalogCount{Count: 5}, false)
}

// @OnEvent
func (catalog *CatalogService) Changed(value CatalogCount) {
	catalog.Count = value.Count
}

// @OnStop
func (catalog *CatalogService) Cleanup() {
	catalog.Count = 0
}
