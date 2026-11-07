import models.Ticket

@RequestRoute("/events")
@Public
class EventsController {
    @Needs
    val tickets: TicketService
    @StreamRoute
    fun streamEvents(): Sequence<Ticket> = ticketsFor(tickets.levels())

    private fun ticketsFor(levels: List<Int>): Sequence<Ticket> = sequence {
        for (level in levels) {
            yield(Ticket("anvil", level))
        }
    }
}
