/** @import { Ticket } from '/models/Ticket.ts' */
@RequestRoute("/events")
@Public
class EventsController {
  /** @type {TicketService} */
  @Needs
  ticketService;
  /** @returns {Generator<Ticket>} */
  @StreamRoute
  *streamEvents() {
    yield* this._ticketsFor(this.ticketService.levels());
  }

  /**
   * @param {number[]} levels
   * @returns {Generator<Ticket>}
   */
  *_ticketsFor(levels) {
    for (const level of levels) {
      yield { sku: 'anvil', level };
    }
  }
}
