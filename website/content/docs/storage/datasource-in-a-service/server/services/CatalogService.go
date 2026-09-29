// @Service
type CatalogService struct {
	// @Needs
	dataSource DataSource
}

func (service *CatalogService) Find() (any, error) {
	return service.dataSource.Query("SELECT 42 AS amount", nil)
}
