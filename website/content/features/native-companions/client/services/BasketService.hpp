#pragma once
#include <string>

class [[Service]] BasketService {
public:
    int count = 0;
    std::string label;

    [[OnStart]]
    void start() {
        count = 6;
        label = "C++";
    }

    int doubled() const { return count * 2; }
};
