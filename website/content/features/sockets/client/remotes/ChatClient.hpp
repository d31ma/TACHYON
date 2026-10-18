#pragma once
#include "../models/ChatMessage.hpp"

class [[HTTPClient("/chat")]] ChatClient {
public:
    [[SocketRequest]] void receive(const ChatMessage& message);
};
