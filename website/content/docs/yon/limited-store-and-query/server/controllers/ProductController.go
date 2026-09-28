import "errors"

// @RequestRoute("/products")
// @Public
type ProductController struct{}

// @GetRoute
func (controller *ProductController) GetProduct() (ProductReport, error) {
	options := NewBehaviorOptions("public-catalog", NewBehaviorSignal())
	report, err := controller.reportFrom(BehaviorsCreate[CatalogSetup](options))
	return report, controller.asResourceError(err)
}

func (controller *ProductController) reportFrom(context *BehaviorContext, failure error) (ProductReport, error) {
	if failure != nil {
		return ProductReport{}, failure
	}
	defer context.Dispose()
	return controller.listProducts(&ProductStore{behaviors: context})
}

func (controller *ProductController) listProducts(products *ProductStore) (ProductReport, error) {
	page, pageErr := products.Bounded(map[string]any{"size": 5})
	activeStatus := "active"
	active, activeErr := products.FindByStatus(activeStatus)
	activeCount, countErr := products.CountByStatus(activeStatus)
	others, othersErr := products.Excluding(activeStatus, page)
	report := ProductReport{Active: active, Count: activeCount, Others: others}
	return report, errors.Join(pageErr, activeErr, countErr, othersErr)
}

func (controller *ProductController) asResourceError(failure error) error {
	if failure != nil {
		return YonResourceError{}
	}
	return nil
}
