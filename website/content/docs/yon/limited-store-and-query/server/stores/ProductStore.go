// @Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})
type ProductStore interface {
	FindByStatus(status BehaviorValue) (BehaviorValue, error)
	CountByStatus(status BehaviorValue) (BehaviorValue, error)
	// @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
	Excluding(
		status BehaviorValue,
		// @PageDefaults
		page BehaviorValue,
	) (BehaviorValue, error)
}
