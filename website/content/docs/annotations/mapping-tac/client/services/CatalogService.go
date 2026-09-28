import "fmt"

type CatalogService struct {
	Title     string
	behaviors *BehaviorContext
}

// @OnStart
func (catalog *CatalogService) Start() error {
	catalog.Title = "Not mapped."
	context, err := BehaviorsCreate[CatalogSetup](NewBehaviorOptions("public-catalog", NewBehaviorSignal()))
	catalog.behaviors = context
	return err
}

func (catalog *CatalogService) Load() error {
	mapper := &ProductConverter{behaviors: catalog.behaviors}
	product := map[string]any{"detail": map[string]any{"id": 7}, "name": "Anvil", "extra": "dropped"}
	return catalog.showSummary(mapper.ToSummary(product))
}

// @OnStop
func (catalog *CatalogService) Stop() {
	catalog.behaviors.Dispose()
}

func (catalog *CatalogService) showSummary(summary BehaviorValue, failure error) error {
	if failure != nil {
		return failure
	}
	fields := summary.(map[string]any)
	catalog.Title = fmt.Sprintf("%v %v: %v", fields["kind"], fields["productId"], fields["title"])
	return nil
}
