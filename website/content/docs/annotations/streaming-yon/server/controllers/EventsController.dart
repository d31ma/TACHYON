import 'models/Ticket.dart';

@RequestRoute("/events")
@Public
class EventsController {
  @Needs
  final TicketService tickets;

  @StreamRoute
  Stream<Ticket> streamEvents() async* {
    yield* _ticketsFor(tickets.levels());
  }

  Stream<Ticket> _ticketsFor(List<int> levels) async* {
    for (final level in levels) {
      yield Ticket('anvil', level);
    }
  }
}
