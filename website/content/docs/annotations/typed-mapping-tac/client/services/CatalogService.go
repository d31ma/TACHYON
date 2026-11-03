import "fmt"

// @Service
type CatalogService struct {
	Result string
	// @Needs
	productConverter *ProductConverter
}

func (catalog *CatalogService) Load() error {
	detail := Detail{Name: "detail", Secret: "detail-private"}
	product := Product{ID: 7, Name: "Anvil", Detail: detail, Items: []Detail{detail}, Secret: "product-private"}
	copied, failure := catalog.productConverter.Copy(product)
	summary, failure := catalog.mapCopy(copied, failure)
	return catalog.showSummary(summary, failure)
}

func (catalog *CatalogService) mapCopy(product Product, failure error) (Summary, error) {
	if failure != nil {
		return Summary{}, failure
	}
	return catalog.productConverter.Map(product)
}

func (catalog *CatalogService) showSummary(summary Summary, failure error) error {
	if failure != nil {
		return failure
	}
	catalog.Result = fmt.Sprintf("%s %d: %s, %s, %d", summary.Kind, summary.ID, summary.Name, summary.Detail.Name, len(summary.Items))
	return nil
}
