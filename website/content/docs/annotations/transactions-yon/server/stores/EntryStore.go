// @Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
type EntryStore struct {
	// @Needs
	behaviors *BehaviorContext
}

func (repository *EntryStore) FindByStatus(status BehaviorValue) (BehaviorValue, error) {}

func (repository *EntryStore) CountByStatus(status BehaviorValue) (BehaviorValue, error) {}

// @AllOrNothing({"adapter":"ledger"})
func (repository *EntryStore) Settle(status BehaviorValue) (BehaviorValue, error) {
	open, err := repository.FindByStatus(status)
	if err != nil {
		return nil, err
	}
	total, err := repository.CountByStatus(status)
	if err != nil {
		return nil, err
	}
	return map[string]any{"open": open, "total": total}, nil
}
