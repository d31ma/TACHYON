#pragma once

class [[Service]] CounterService {
public:
    int count = 0;

    void increment() { count += 1; }
};
