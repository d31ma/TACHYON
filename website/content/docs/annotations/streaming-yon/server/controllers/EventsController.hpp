#pragma once
#include <vector>
#include "../models/Ticket.hpp"

class [[RequestRoute("/events")]] [[Public]] EventsController {
    [[Needs]] TicketService& tickets;

public:
    [[StreamRoute]]
    tachyon::Generator<Ticket> streamEvents() {
        return ticketsFor(tickets.levels());
    }

private:
    tachyon::Generator<Ticket> ticketsFor(std::vector<int> levels) {
        for (int level : levels) {
            co_yield Ticket{"anvil", level};
        }
    }
};
