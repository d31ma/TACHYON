// @Service
type MenuService struct {
	Expanded bool
}

func (menu *MenuService) Toggle() {
	menu.Expanded = !menu.Expanded
}
