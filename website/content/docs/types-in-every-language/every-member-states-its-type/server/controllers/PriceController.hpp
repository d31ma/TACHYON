#pragma once
#include "../models/Price.hpp"

class [[RequestRoute("/price")]] [[Public]] PriceController {
    [[Needs]] PriceService& priceService;

public:
    [[GetRoute]]
    Price getPrice() {
        return priceService.priceFor(20, 3);
    }
};
