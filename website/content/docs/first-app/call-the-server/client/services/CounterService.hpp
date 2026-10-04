#pragma once
#include <string>
#include "../remotes/GreetingClient.hpp"

class [[Service]] CounterService {
public:
    int count = 0;
    std::string message;
    [[Needs]] GreetingClient& greetingClient;

    void increment() { count += 1; }

    Task<void> askServer() {
        Greeting greeting = co_await greetingClient.getGreeting();
        message = greeting.message;
    }
};
