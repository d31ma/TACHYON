#pragma once
#include <string>

class [[Service]] TallyService {
public:
    int count = 0;

    int add(int amount) {
        count += amount;
        return count;
    }

    std::string summary(const std::string& unit) const {
        return std::to_string(count) + " " + unit;
    }
};
