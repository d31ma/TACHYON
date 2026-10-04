#pragma once
#include "../models/CartTotal.hpp"

class [[Service]] LiveTotalService {
public:
    int seen = 0;

    [[OnEvent({"replay":true})]]
    void changed(const CartTotal& value) { seen = value.total; }
};
