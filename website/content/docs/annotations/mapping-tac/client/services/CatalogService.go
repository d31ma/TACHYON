import "fmt"

type CatalogService struct {
	Title string
	// @Needs
	converter *ProductConverter
}

// @OnStart
func (catalog *CatalogService) Start() {
	catalog.Title = "Not mapped."
}

func (catalog *CatalogService) Load() error {
	product := map[string]any{"detail": map[string]any{"id": 7}, "name": "Anvil", "extra": "dropped"}
	return catalog.showSummary(catalog.converter.ToSummary(product))
}

func (catalog *CatalogService) showSummary(summary BehaviorValue, failure error) error {
	if failure != nil {
		return failure
	}
	fields := summary.(map[string]any)
	catalog.Title = fmt.Sprintf("%v %v: %v", fields["kind"], fields["productId"], fields["title"])
	return nil
}
