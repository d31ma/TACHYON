#pragma once

class [[Service]] BillingService {
public:
    int total = 0;

    [[OnStart]] void start() {
        total = 42;
    }
};
