#pragma once
#include "../models/Money.hpp"

class [[RequestRoute("/money")]] [[Public]] MoneyController {
    [[Needs]] MoneyService& money;

public:
    [[GetRoute]]
    Money getMoney() {
        return Money{money.amount()};
    }
};
