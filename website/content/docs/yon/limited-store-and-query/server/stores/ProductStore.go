// @Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
type ProductStore struct {
	// @Needs
	behaviors *BehaviorContext
}

func (repository *ProductStore) FindByStatus(status BehaviorValue) (BehaviorValue, error) {}

func (repository *ProductStore) CountByStatus(status BehaviorValue) (BehaviorValue, error) {}

// @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
func (repository *ProductStore) Excluding(
	status BehaviorValue,
	// @PageDefaults
	page BehaviorValue,
) (BehaviorValue, error) {
}

func (repository *ProductStore) Bounded(
	// @PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})
	request BehaviorValue,
) (BehaviorValue, error) {
	return request, nil
}
