#pragma once
#include "../models/CountChanged.hpp"

class [[Service]] CounterService {
public:
    int count = 0;
    ApplicationEventPublisher applicationEventPublisher;

    void add() {
        count += 1;
        applicationEventPublisher.publishEvent(CountChanged{count}, false);
    }
};
