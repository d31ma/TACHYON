#pragma once
#include <string>

class [[Service]] TaxService {
public:
    int rate(const std::string& region) const {
        return 2;
    }

    int total(int count) const {
        return count * 10;
    }
};
