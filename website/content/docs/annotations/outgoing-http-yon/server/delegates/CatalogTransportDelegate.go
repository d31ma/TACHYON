// @Delegate
type CatalogTransportDelegate struct{}

func (helper *CatalogTransportDelegate) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	return map[string]any{"items": []any{"anvil"}}, nil
}
