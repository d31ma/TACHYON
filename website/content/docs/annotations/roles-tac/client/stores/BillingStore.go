// @Store(BillingEntity, {"fields":["id"],"defaultSize":20,"maxSize":100})
type BillingStore interface {
	ExistsById(id BehaviorValue) (BehaviorValue, error)
}
