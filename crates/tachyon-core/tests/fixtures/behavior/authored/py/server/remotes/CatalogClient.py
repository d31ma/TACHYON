from typing import Annotated, Protocol

@cache_settings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
@http_client({"adapter": "api"})
class CatalogClient(Protocol):
    @cached
    @get_request("/products")
    async def remote(self) -> object: ...
    @get_request("/products")
    async def search(self, page: Annotated[object, request_param("page")]) -> object: ...
    @get_request("/products/{id}")
    async def find(self, id: Annotated[object, path_variable]) -> object: ...
