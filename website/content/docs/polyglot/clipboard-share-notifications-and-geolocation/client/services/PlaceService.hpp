#pragma once
#include <stdexcept>
#include <string>

class [[Service]] PlaceService {
public:
    std::string place;
    std::string problem;
    [[Needs]] Geolocation& geolocation;
    [[Needs]] Clipboard& clipboard;

    void locate() {
        try {
            geolocation.currentPosition();
        } catch (const std::runtime_error& refusal) {
            problem = refusal.what();
        }
    }

    void copy() {
        try {
            clipboard.writeText(place);
        } catch (const std::runtime_error& refusal) {
            problem = refusal.what();
        }
    }

    [[OnEvent]]
    void found(const PositionFound& position) {
        place = std::to_string(position.latitude) + ", " + std::to_string(position.longitude);
    }

    [[OnEvent]]
    void failed(const BrowserCallFailed& failure) { problem = failure.message; }
};
