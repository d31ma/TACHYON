from typing import Protocol

from models import Greeting


@http_client("/api")
class OverrideClient(Protocol):
    @get_request("/never")
    async def read(self) -> Greeting: ...
