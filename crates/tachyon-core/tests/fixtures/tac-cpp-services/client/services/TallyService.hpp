#pragma once
#include <string>
#include "CounterService.hpp"
#include "../models/PlacedOrder.hpp"

class [[Service]] TallyService {
public:
    int count = 0;
    std::string label = "Tally";
    [[Needs]] CounterService& counter;
    ApplicationEventPublisher applicationEventPublisher;

    void add(int amount) { count += amount; }

    int ticket() { return counter.next(); }

    PlacedOrder place(int total) {
        PlacedOrder order{"paid", total, {"from C++"}};
        applicationEventPublisher.publishEvent(order);
        return order;
    }

    PlacedOrder doubled(const PlacedOrder& order) {
        PlacedOrder copy = order;
        copy.total = order.total * 2;
        return copy;
    }
};
