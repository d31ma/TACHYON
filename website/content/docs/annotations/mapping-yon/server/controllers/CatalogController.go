// @RequestRoute("/catalog")
// @Public
type CatalogController struct{}

// @GetRoute
func (controller *CatalogController) GetCatalog() (CatalogSummary, error) {
	options := NewBehaviorOptions("public-catalog", NewBehaviorSignal())
	summary, err := controller.summaryFrom(BehaviorsCreate[CatalogSetup](options))
	return CatalogSummary{Summary: summary}, controller.asResourceError(err)
}

func (controller *CatalogController) summaryFrom(context *BehaviorContext, failure error) (BehaviorValue, error) {
	if failure != nil {
		return nil, failure
	}
	defer context.Dispose()
	mapper := &ProductConverter{behaviors: context}
	product := map[string]any{
		"detail": map[string]any{"id": 7},
		"name":   "Anvil",
		"extra":  "dropped",
	}
	return mapper.ToSummary(product)
}

func (controller *CatalogController) asResourceError(failure error) error {
	if failure != nil {
		return YonResourceError{}
	}
	return nil
}
