from typing import Protocol

from models import Greeting


@http_client("/api")
class GreetingClient(Protocol):
    @get_request("/greeting")
    async def get_greeting(self) -> Greeting: ...
