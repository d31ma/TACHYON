#pragma once
#include <string>

class [[Service]] CounterService {
public:
    int count = 0;
    [[SessionState]] int visits = 0;
    [[LocalState]] std::string theme = "light";

    void increment() { count += 1; }
};
