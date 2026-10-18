from typing import Annotated, ClassVar

from models import ChatMessage


@service
class ChatService:
    MESSAGE_LIMIT: ClassVar[int] = 50

    chat_client: Annotated[ChatClient, needs]
    messages: list[ChatMessage] = []

    @on_event
    def add_message(self, message: ChatMessage) -> None:
        self.messages = (self.messages + [message])[-self.MESSAGE_LIMIT:]

    async def say_hello(self) -> None:
        await self.chat_client.send(ChatMessage("Hello from a tab"))
