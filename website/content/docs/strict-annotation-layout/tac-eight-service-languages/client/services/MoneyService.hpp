#pragma once

class [[Service]] MoneyService {
public:
    long long amount = 0;

    [[OnStart]] void start() {
        amount = 42;
    }
};
