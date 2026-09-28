// @Store
type OrderStore struct{}

func (repository *OrderStore) All() []string {
	return []string{"anvil"}
}
