#pragma once
#include "../models/CatalogCount.hpp"

class [[Service]] CatalogService {
public:
    int count = 0;
    ApplicationEventPublisher applicationEventPublisher;

    void announce() {
        applicationEventPublisher.publishEvent(CatalogCount{5});
    }

    [[OnStart]]
    void initialize() { announce(); }

    [[OnEvent]]
    void changed(const CatalogCount& value) { count = value.count; }

    [[OnStop]]
    void cleanup() { count = 0; }
};
