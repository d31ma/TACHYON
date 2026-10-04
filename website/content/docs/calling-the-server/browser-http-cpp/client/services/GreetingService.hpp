#pragma once
#include <string>
#include "../remotes/GreetingClient.hpp"

class [[Service]] GreetingService {
public:
    std::string message;
    int loads = 0;
    [[Needs]] GreetingClient& greetingClient;

    Task<void> ask() {
        loads += 1;
        Greeting greeting = co_await greetingClient.read();
        message = greeting.message;
    }
};
