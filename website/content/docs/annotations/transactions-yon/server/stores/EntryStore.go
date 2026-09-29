// @Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})
type EntryStore interface {
	FindByStatus(status BehaviorValue) (BehaviorValue, error)
	CountByStatus(status BehaviorValue) (BehaviorValue, error)
}
