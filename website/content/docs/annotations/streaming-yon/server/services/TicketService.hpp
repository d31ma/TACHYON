#pragma once
#include <vector>

class [[Service]] TicketService {
public:
    std::vector<int> levels() const { return {1, 2, 3}; }
};
