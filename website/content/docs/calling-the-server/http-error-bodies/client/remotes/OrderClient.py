from typing import Annotated, Protocol

from models import OrderInput


@http_client("/api")
class OrderClient(Protocol):
    @post_request("/orders")
    async def send(self, order: Annotated[dict[str, object], request_body]) -> OrderInput: ...
