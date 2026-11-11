// @Service
type RenameService struct {
	Editing bool
	Problem string
	// @Needs
	focus Focus
}

func (service *RenameService) Open() {
	service.Editing = true
	service.keepProblem(service.focus.Trap("dialog"))
	service.keepProblem(service.focus.Move("name"))
}

func (service *RenameService) Close() {
	service.Editing = false
	service.keepProblem(service.focus.Release())
}

// @OnEvent
func (service *RenameService) Failed(failed BrowserCallFailed) {
	service.Problem = failed.Message
}

func (service *RenameService) keepProblem(problem error) {
	if problem != nil {
		service.Problem = problem.Error()
	}
}
