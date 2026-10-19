import models.Ticket

@RequestRoute("/events")
@Public
class EventsController {
    @Needs
    final TicketService tickets
    @StreamRoute
    Iterator<Ticket> streamEvents() {
        return ticketsFor(tickets.levels())
    }

    private Iterator<Ticket> ticketsFor(List<Integer> levels) {
        return levels.collect { level -> new Ticket("anvil", level) }.iterator()
    }
}
