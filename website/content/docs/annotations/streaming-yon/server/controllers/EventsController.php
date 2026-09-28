<?php
use Models\Ticket;

#[RequestRoute("/events")]
#[Public]
class EventsController {
    #[Needs]
    public TicketService $tickets;
    #[StreamRoute]
    public function streamEvents(): Generator {
        yield from $this->ticketsFor($this->tickets->levels());
    }

    private function ticketsFor(array $levels): Generator {
        foreach ($levels as $level) {
            yield new Ticket('anvil', $level);
        }
    }
}
