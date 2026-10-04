#pragma once
#include "../services/CounterService.hpp"

class [[Test(HelloTestSetup)]] CounterServiceTest {
public:
    void testIncrement() {
        CounterService counter;
        counter.increment();
        Assert::equals(1, counter.count, "the count");
    }
};
