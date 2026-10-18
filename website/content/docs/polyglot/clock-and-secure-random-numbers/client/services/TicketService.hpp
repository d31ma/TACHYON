#pragma once
#include <string>

class [[Service]] TicketService {
public:
    std::string ticket;
    double issuedAt = 0;
    [[Needs]] Clock& clock;
    [[Needs]] SecureRandom& secureRandom;

    void issue() {
        ticket = secureRandom.uuid();
        issuedAt = clock.nowMs();
    }
};
