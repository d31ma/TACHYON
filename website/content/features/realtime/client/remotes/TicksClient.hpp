#pragma once
#include "../models/Tick.hpp"

class [[HTTPClient("/ticks")]] TicksClient {
public:
    [[StreamRequest]] void receive(const Tick& tick);
};
