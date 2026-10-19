from typing import Annotated, Protocol

from models import SentRequest


@http_client({"path":"/api","adapter":"orders"})
class OrdersClient(Protocol):
    @put_request("/orders/{id}")
    async def replace(self, id: Annotated[str, path_variable("id")], order: Annotated[dict[str, str], request_body], tenant: Annotated[str, request_header("X-Tenant")]) -> SentRequest: ...

    @patch_request("/orders/{id}")
    async def change(self, id: Annotated[str, path_variable("id")], change: Annotated[dict[str, str], request_body], tenant: Annotated[str, request_header("X-Tenant")]) -> SentRequest: ...

    @delete_request("/orders/{id}")
    async def remove(self, id: Annotated[str, path_variable("id")], reason: Annotated[str, request_header("X-Reason")]) -> SentRequest: ...
