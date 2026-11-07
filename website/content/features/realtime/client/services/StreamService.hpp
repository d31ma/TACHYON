#pragma once
#include <cstddef>
#include <vector>
#include "../models/Tick.hpp"
#include "../remotes/TicksClient.hpp"

class [[Service]] StreamService {
public:
    [[Needs]] TicksClient& ticksClient;
    std::vector<Tick> ticks;

    [[OnEvent]] void addTick(const Tick& tick) {
        ticks.push_back(tick);
        if (ticks.size() > TickLimit) ticks.erase(ticks.begin());
    }

private:
    static constexpr std::size_t TickLimit = 128;
};
