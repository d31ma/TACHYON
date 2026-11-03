#pragma once
#include "../models/PlaceFound.hpp"

class [[Service]] MapService {
public:
    double latitude = 0.0;
    double longitude = 0.0;

    [[OnEvent]]
    void show(const PlaceFound& value) {
        latitude = value.latitude;
        longitude = value.longitude;
    }
};
