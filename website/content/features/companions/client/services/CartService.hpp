#pragma once
#include "../models/CartTotal.hpp"

class [[Service]] CartService {
public:
    int total = 0;
    ApplicationEventPublisher applicationEventPublisher;

    void add() {
        total += 1;
        applicationEventPublisher.publishEvent(CartTotal{total}, true);
    }
};
