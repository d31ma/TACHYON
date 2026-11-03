import "iter"

// @RequestRoute("/events")
// @Public
type EventsController struct {
	// @Needs
	tickets *TicketService
}

// @StreamRoute
func (controller *EventsController) StreamEvents() iter.Seq[Ticket] {
	return controller.ticketsFor(controller.tickets.Levels())
}

func (controller *EventsController) ticketsFor(levels []int) iter.Seq[Ticket] {
	return func(yield func(Ticket) bool) {
		for _, level := range levels {
			if !yield(Ticket{Sku: "anvil", Level: level}) {
				return
			}
		}
	}
}
