// @Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"ConfigSetup"})
type ProductQueriesStore interface {
	Repository[ProductEntity, string]
	FindByStatus(status BehaviorValue) (BehaviorValue, error)
	DeleteByStatus(status BehaviorValue) (BehaviorValue, error)
	CountByStatus(status BehaviorValue) (BehaviorValue, error)
	ExistsById(id BehaviorValue) (BehaviorValue, error)
	// @Query({"operation":"find","where":[{"field":"status","operator":"eq","argument":0}],"orderBy":[]})
	Selected(
		status BehaviorValue,
		// @PageDefaults
		page BehaviorValue,
	) (BehaviorValue, error)
}
