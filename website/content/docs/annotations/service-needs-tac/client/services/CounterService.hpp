#pragma once

class [[Service]] CounterService {
public:
    int count = 0;

    int increment() {
        count += 1;
        return count;
    }
};
