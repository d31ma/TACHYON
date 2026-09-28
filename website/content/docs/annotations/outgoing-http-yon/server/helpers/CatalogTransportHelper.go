// @Helper
type CatalogTransportHelper struct{}

func (helper *CatalogTransportHelper) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	return map[string]any{"items": []any{"anvil"}}, nil
}
