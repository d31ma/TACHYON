import java.util.List;
import java.util.stream.Stream;
import models.Ticket;

@RequestRoute("/events")
@Public
class EventsController {
    @Needs
    final TicketService tickets;
    @StreamRoute
    Stream<Ticket> streamEvents() {
        return ticketsFor(tickets.levels());
    }

    private Stream<Ticket> ticketsFor(List<Integer> levels) {
        return levels.stream().map(level -> new Ticket("anvil", level));
    }
}
