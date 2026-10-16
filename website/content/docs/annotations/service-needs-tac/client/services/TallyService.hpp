#pragma once
#include "CounterService.hpp"

class [[Service]] TallyService {
public:
    int total = 0;
    [[Needs]] CounterService& counter;

    void bump() { total = counter.increment(); }
};
