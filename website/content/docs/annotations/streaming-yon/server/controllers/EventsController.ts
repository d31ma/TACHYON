import type { Ticket } from '/models/Ticket.ts'

@RequestRoute("/events")
@Public
class EventsController {
  @Needs
  readonly tickets!: TicketService;
  @StreamRoute
  *streamEvents(): Generator<Ticket> {
    yield* this.ticketsFor(this.tickets.levels());
  }

  private *ticketsFor(levels: number[]): Generator<Ticket> {
    for (const level of levels) {
      yield { sku: 'anvil', level };
    }
  }
}
