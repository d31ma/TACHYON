#pragma once
#include <string>
#include <vector>
#include "../models/Tab.hpp"

class [[Service]] BasketService {
public:
    int count = 0;
    std::string label;
    std::vector<Tab> tabs{{"a", "A"}};

    void increment() { count += 1; }

    void select(const std::string& id) { selected = id; }

private:
    std::string selected;
};
