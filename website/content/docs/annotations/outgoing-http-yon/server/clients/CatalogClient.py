from typing import Annotated, Protocol

@http_client
class CatalogClient(Protocol):
    @get_request("/products")
    async def all(self, page: Annotated[int, request_param("page")]) -> object: ...

    @post_request("/products")
    async def add(self, product: Annotated[object, request_body]) -> object: ...
