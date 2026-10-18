from typing import Annotated


@service
class CounterService:
    greeting_client: Annotated[GreetingClient, needs]
    count: int = 0
    message: str = ''

    def increment(self) -> None:
        self.count += 1

    async def ask_server(self) -> None:
        greeting = await self.greeting_client.get_greeting()
        self.message = greeting.message
