import "errors"

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
	catalog.behaviors = context
	offset, pageErr := catalog.Page(map[string]any{"page": 1})
	catalog.Offset = catalog.offsetOf(offset)
	return errors.Join(err, pageErr)
}

func (catalog *CatalogService) offsetOf(value BehaviorValue) int64 {
	number, _ := value.(float64)
	return int64(number)
}
