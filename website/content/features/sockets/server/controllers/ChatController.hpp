#pragma once
#include <vector>
#include "../models/ChatMessage.hpp"

class [[SocketRoute("/chat")]] [[Public]] ChatController {
public:
    [[OnOpen]]
    void onOpen(SocketSession& session) {
        session.send(ChatMessage{"Welcome to the room"});
    }

    [[OnMessage]]
    void onMessage(SocketSession& session, ChatMessage message) {
        sendToEveryone(session.openSessions(), message);
    }

private:
    void sendToEveryone(const std::vector<SocketPeer>& peers, const ChatMessage& message) {
        for (const SocketPeer& peer : peers) {
            peer.send(message);
        }
    }
};
