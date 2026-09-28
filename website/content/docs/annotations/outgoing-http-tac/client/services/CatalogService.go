// @Service
// @HTTPClient
type CatalogService struct {
	behaviors *BehaviorContext
	Count     int
}

// @OnStart
func (catalog *CatalogService) Start() error {
	options := NewBehaviorOptions("catalog", NewBehaviorSignal())
	context, err := BehaviorsCreate[CatalogSetup](options)
	catalog.behaviors = context
	return err
}

// @GetRequest("/api/products")
func (catalog *CatalogService) All() (BehaviorValue, error) {
	return map[string]any{}, nil
}
