#pragma once

class [[Service]] PublishService {
    ApplicationEventPublisher applicationEventPublisher;

public:
    [[OnStart]]
    void initialize() { applicationEventPublisher.publishEvent(ProbeReady{7}); }
};
