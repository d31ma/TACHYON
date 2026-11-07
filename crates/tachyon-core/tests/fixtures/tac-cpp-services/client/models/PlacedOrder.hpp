#pragma once
#include <string>
#include <vector>

struct PlacedOrder {
    std::string status;
    int total = 0;
    std::vector<std::string> notes;
};
