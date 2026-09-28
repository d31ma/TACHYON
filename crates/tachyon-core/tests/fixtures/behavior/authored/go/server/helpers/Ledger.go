type Ledger struct {
	// @Needs
	behaviors *BehaviorContext
}

// @AllOrNothing({"adapter":"data"})
func (ledger *Ledger) Post(id BehaviorValue) (BehaviorValue, error) {
	repository, err := BehaviorsRepository[ProductEntity](ledger.behaviors)
	if err != nil {
		return nil, err
	}
	return repository.Save(map[string]any{"id": id})
}

// @AllOrNothing({"adapter":"data"})
func (ledger *Ledger) Refuse(id BehaviorValue) (BehaviorValue, error) {
	repository, err := BehaviorsRepository[ProductEntity](ledger.behaviors)
	if err != nil {
		return nil, err
	}
	if _, err := repository.Save(map[string]any{"id": id}); err != nil {
		return nil, err
	}
	return nil, &BehaviorError{Code: "validation", Message: "the ledger refused the entry"}
}
