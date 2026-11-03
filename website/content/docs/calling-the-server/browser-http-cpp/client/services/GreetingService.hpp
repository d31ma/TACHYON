#pragma once
#include <string>
#include "../remotes/GreetingClient.hpp"
#include "../remotes/OverrideClient.hpp"

class [[Service]] GreetingService {
public:
    std::string message;
    std::string custom;
    int loads = 0;
    int clears = 0;
    [[Needs]] GreetingClient& greetingClient;
    [[Needs]] OverrideClient& overrideClient;

    Task<void> ask() {
        loads += 1;
        Greeting greeting = co_await greetingClient.read();
        message = greeting.message;
    }

    Task<void> clear() {
        co_await greetingClient.clear();
        clears += 1;
    }

    Task<void> askCustom() {
        Greeting greeting = co_await overrideClient.read();
        custom = greeting.message;
    }
};
