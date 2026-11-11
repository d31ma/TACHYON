#pragma once

class [[Service]] TickerService {
public:
    int seconds = 0;

    [[Every("1s")]] void count() { seconds += 1; }
};
