// @Store
type CatalogStore struct {
	// @Needs
	dataSource DataSource
}

func (repository *CatalogStore) Find() (any, error) {
	return repository.dataSource.Query("SELECT 42 AS amount", nil)
}
