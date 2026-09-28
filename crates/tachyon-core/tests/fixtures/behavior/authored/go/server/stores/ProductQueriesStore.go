// @Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
type ProductQueriesStore struct {
	// @Needs
	behaviors *BehaviorContext
}

func (repository *ProductQueriesStore) FindByStatus(status BehaviorValue) (BehaviorValue, error) {}

func (repository *ProductQueriesStore) CountByStatus(status BehaviorValue) (BehaviorValue, error) {}

func (repository *ProductQueriesStore) ExistsById(id BehaviorValue) (BehaviorValue, error) {}

// @Query({"operation":"find","where":[{"field":"status","operator":"eq","argument":0}],"orderBy":[]})
func (repository *ProductQueriesStore) Selected(
	status BehaviorValue,
	// @PageDefaults
	page BehaviorValue,
) (BehaviorValue, error) {
}
