// @RequestRoute("/events")
// @Public
type EventsController struct {
	// @Needs
	eventService *EventService
}

// @GetRoute
func (controller *EventsController) GetEvents() (EventReport, error) {
	return controller.eventService.Report()
}
