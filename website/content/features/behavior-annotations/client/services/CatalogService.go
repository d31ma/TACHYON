// @Service
type CatalogService struct {
	behaviors *BehaviorContext
	Offset    int64
}

func (catalog *CatalogService) Page(
	// @PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})
	input BehaviorValue,
) (BehaviorValue, error) {
	return input.(map[string]any)["offset"], nil
}

// @OnStart
func (catalog *CatalogService) Initialize() error {
	context, err := BehaviorsCreate[ApplicationSetup](NewBehaviorOptions("public-catalog", NewBehaviorSignal()))
	if err != nil {
		return err
	}
	catalog.behaviors = context
	offset, err := catalog.Page(map[string]any{"page": 1})
	if number, ok := offset.(float64); ok {
		catalog.Offset = int64(number)
	}
	return err
}
