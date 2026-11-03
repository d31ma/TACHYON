#pragma once

class [[Service]] CounterService {
public:
    int value = 100;

    int next() {
        value += 1;
        return value;
    }
};
