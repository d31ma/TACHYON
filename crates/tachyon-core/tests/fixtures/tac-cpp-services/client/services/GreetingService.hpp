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

    Task<std::string> shout(std::string text) {
        Greeting answer = co_await greetingClient.echo(7, Greeting{text});
        co_return answer.message + "!";
    }

    Task<std::string> lost() {
        Greeting answer = co_await greetingClient.lost();
        co_return answer.message;
    }
};
