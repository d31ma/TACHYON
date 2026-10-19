// @Service
type ActivityService struct {
	State string
}

// @OnEvent({"replay":true})
func (activity *ActivityService) ApplicationStateChanged(changed ApplicationStateChanged) {
	activity.State = changed.State
}
