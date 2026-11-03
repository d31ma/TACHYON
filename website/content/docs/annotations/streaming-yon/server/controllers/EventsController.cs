using Models;

[RequestRoute("/events")]
[Public]
class EventsController {
    [Needs]
    readonly TicketService _tickets;
    [StreamRoute]
    public System.Collections.Generic.IEnumerable<Ticket> StreamEvents() {
        return TicketsFor(_tickets.levels());
    }

    private static System.Collections.Generic.IEnumerable<Ticket> TicketsFor(int[] levels) {
        foreach (var level in levels) {
            yield return new Ticket("anvil", level);
        }
    }
}
