// @Service
type MenuService struct {
	Expanded bool
}

func (service *MenuService) Toggle() {
	service.Expanded = !service.Expanded
}

func (service *MenuService) Close() {
	service.Expanded = false
}
