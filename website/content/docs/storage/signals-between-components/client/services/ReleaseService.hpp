#pragma once
#include <string>
#include "../models/ReleaseChanged.hpp"

class [[Service]] ReleaseService {
public:
    std::string release;
    ApplicationEventPublisher applicationEventPublisher;

    void announce() {
        applicationEventPublisher.publishEvent(ReleaseChanged{"TACHYON"}, true);
    }

    [[OnEvent({"replay":true})]]
    void receive(const ReleaseChanged& changed) { release = changed.release; }
};
