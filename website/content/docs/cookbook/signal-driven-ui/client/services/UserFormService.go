// @Service
type UserFormService struct {
	// @Needs
	usersClient               *UsersClient
	applicationEventPublisher ApplicationEventPublisher
	Name                      string
}

func (form *UserFormService) Save() error {
	saved, err := form.usersClient.Save(form.Name)
	form.applicationEventPublisher.PublishEvent(saved, false)
	return err
}
