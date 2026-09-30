// @Service
type BadgeService struct {
	Status string
}

// @OnStart
func (badge *BadgeService) Start() {
	badge.Status = "ready"
}
