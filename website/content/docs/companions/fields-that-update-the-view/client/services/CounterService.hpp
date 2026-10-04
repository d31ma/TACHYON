#pragma once
#include <string>

class [[Service]] CounterService {
public:
    int count = 0;
    std::string theme = "light";

    void increment() { count += 1; }
};
