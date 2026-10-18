#pragma once

class [[Service]] CheckoutService {
    ApplicationEventPublisher applicationEventPublisher;

public:
    [[OnStart]]
    void start() { applicationEventPublisher.publishEvent(PlacedOrder{"paid", 25}); }

    [[OnStop]]
    void stop() {}
};
