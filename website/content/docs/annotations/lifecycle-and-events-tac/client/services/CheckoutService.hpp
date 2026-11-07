#pragma once
#include "../models/PlacedOrder.hpp"

class [[Service]] CheckoutService {
public:
    int placed = 0;
    ApplicationEventPublisher applicationEventPublisher;

    void place() {
        placed += 1;
        applicationEventPublisher.publishEvent(PlacedOrder{"paid", 25}, true);
    }

private:
    [[OnStart]]
    void start() { place(); }

    [[OnStop]]
    void stop() { placed = 0; }
};
