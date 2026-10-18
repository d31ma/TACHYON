#pragma once
#include <stdexcept>

class [[Helper]] PriceHelper {
public:
    int withTax(int cents) const {
        requirePositive(cents);
        return cents * 120 / 100;
    }

private:
    void requirePositive(int cents) const {
        if (cents < 0) {
            throw std::invalid_argument("A price is never negative.");
        }
    }
};
