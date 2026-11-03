#pragma once

class [[Service]] BasketService {
public:
    int count = 0;

    int doubled() const { return count * 2; }

    [[OnStart]]
    void initialize() { count = 6; }
};
