// @Helper
type CatalogAdapterHelper struct{}

func (adapter *CatalogAdapterHelper) Execute(command *BehaviorEntityCommand, execution *BehaviorExecution) (BehaviorValue, error) {
	if err := execution.Check(); err != nil {
		return nil, err
	}
	query, isObject := command.Query.(map[string]any)
	if !isObject {
		return nil, adapter.invalid()
	}
	conditions, isList := query["where"].([]any)
	if !isList || len(conditions) == 0 {
		return nil, adapter.invalid()
	}
	condition, isCondition := conditions[0].(map[string]any)
	if !isCondition {
		return nil, adapter.invalid()
	}
	matchingRows := adapter.matchingRows(condition)
	if query["kind"] == "count" {
		return len(matchingRows), nil
	}
	return matchingRows, nil
}

func (adapter *CatalogAdapterHelper) matchingRows(condition map[string]any) []any {
	keepUnequalRows := condition["operator"] == "ne"
	wantedStatus := condition["value"]
	rows := []map[string]any{{"id": 1, "status": "active"}, {"id": 2, "status": "retired"}}
	matchingRows := []any{}
	for _, row := range rows {
		isEqual := row["status"] == wantedStatus
		isMatch := isEqual
		if keepUnequalRows {
			isMatch = !isEqual
		}
		if !isMatch {
			continue
		}
		matchingRows = append(matchingRows, row)
	}
	return matchingRows
}

func (adapter *CatalogAdapterHelper) invalid() error {
	return &BehaviorError{Code: "invalid", Message: "The query is not valid."}
}
