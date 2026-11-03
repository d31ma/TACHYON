#pragma once
#include <string>

class [[Service]] PriceService {
    static constexpr const char* CURRENCY = "EUR";

public:
    Price priceFor(int amount, int count) const {
        return Price{amount * count, CURRENCY};
    }
};
