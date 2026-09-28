<?php
use Models\ChatMessage;

#[SocketRoute("/chat")]
#[Public]
class ChatController {
    #[OnOpen]
    public function onOpen(SocketSession $session): void {
        $session->send(new ChatMessage('Welcome to the room'));
    }

    #[OnMessage]
    public function onMessage(SocketSession $session, ChatMessage $message): void {
        $this->sendToEveryone($session->openSessions(), $message);
    }

    private function sendToEveryone(array $peers, ChatMessage $message): void {
        foreach ($peers as $peer) {
            $peer->send($message);
        }
    }
}
