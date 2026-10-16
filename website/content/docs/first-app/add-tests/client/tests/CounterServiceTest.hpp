#pragma once
#include "../services/CounterService.hpp"
#include "../remotes/GreetingClient.hpp"

class [[Test(HelloTestSetup)]] CounterServiceTest {
public:
    [[Fake]] GreetingClient greetingClient;

    void testIncrementAddsOne() {
        CounterService counter = construct<CounterService>();
        counter.increment();
        Assert::equals(1, counter.count);
    }

    Task<void> testAskServerSavesTheMessage() {
        CounterService counter = construct<CounterService>();
        co_await counter.askServer();
        Assert::equals("Hi", counter.message);
    }
};
