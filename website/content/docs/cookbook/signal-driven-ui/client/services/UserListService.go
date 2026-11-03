// @Service
type UserListService struct {
	Names []string
}

// @OnEvent
func (list *UserListService) UserSaved(saved UserSaved) {
	list.Names = append(list.Names, saved.Name)
}
