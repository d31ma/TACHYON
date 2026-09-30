import "errors"

// @RequestRoute("/products")
// @Public
type ProductController struct {
	// @Needs
	products *ProductStore
}

// @GetRoute
func (controller *ProductController) GetProduct() (ProductReport, error) {
	activeStatus := "active"
	active, activeErr := controller.products.FindByStatus(activeStatus)
	activeCount, countErr := controller.products.CountByStatus(activeStatus)
	others, othersErr := controller.products.Excluding(activeStatus, map[string]any{"size": 5})
	first, firstErr := controller.products.FindById(1)
	report := ProductReport{Active: active, Count: activeCount, Others: others, First: first}
	return report, controller.asResourceError(errors.Join(activeErr, countErr, othersErr, firstErr))
}

func (controller *ProductController) asResourceError(failure error) error {
	if failure != nil {
		return YonResourceError{}
	}
	return nil
}
