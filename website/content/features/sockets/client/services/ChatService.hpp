#pragma once
#include <cstddef>
#include <vector>
#include "../models/ChatMessage.hpp"
#include "../remotes/ChatClient.hpp"

class [[Service]] ChatService {
public:
    [[Needs]] ChatClient& chatClient;
    std::vector<ChatMessage> messages;

    [[OnEvent]] void addMessage(const ChatMessage& message) {
        messages.push_back(message);
        if (messages.size() > MessageLimit) messages.erase(messages.begin());
    }

    Task<void> sayHello() { co_await chatClient.send(ChatMessage{"Hello from a tab"}); }

private:
    static constexpr std::size_t MessageLimit = 50;
};
