from typing import Protocol

from models import Greeting


@cache_settings({"name":"greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
@http_client("/api")
class GreetingClient(Protocol):
    @cached
    @get_request("/greeting")
    async def read(self) -> Greeting: ...

    @clear_cache({"method":"read"})
    @post_request("/clear")
    async def clear(self) -> Greeting: ...
