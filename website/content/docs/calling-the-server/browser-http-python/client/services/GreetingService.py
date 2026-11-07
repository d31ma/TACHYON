from typing import Annotated


@service
class GreetingService:
    greeting_client: Annotated[GreetingClient, needs]
    override_client: Annotated[OverrideClient, needs]
    message: str = ''
    custom: str = ''
    loads: int = 0
    clears: int = 0

    async def ask(self) -> None:
        self.loads += 1
        greeting = await self.greeting_client.read()
        self.message = greeting.message

    async def clear(self) -> None:
        await self.greeting_client.clear()
        self.clears += 1

    async def ask_custom(self) -> None:
        greeting = await self.override_client.read()
        self.custom = greeting.message
