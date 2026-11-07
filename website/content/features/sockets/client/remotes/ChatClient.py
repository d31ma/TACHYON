from typing import Protocol

from models import ChatMessage


@http_client("/chat")
class ChatClient(Protocol):
    @socket_request
    def receive(self, message: ChatMessage) -> None: ...
