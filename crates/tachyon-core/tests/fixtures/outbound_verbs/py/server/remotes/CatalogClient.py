from typing import Annotated, Protocol

@http_client("/api")
class CatalogClient(Protocol):
    @get_request("/products/{id}")
    async def find(self, id: Annotated[str, path_variable("id")], view: Annotated[str, request_param("view")], tenant: Annotated[str, request_header("X-Tenant")]) -> object: ...

    @post_request("/products")
    async def save(self, product: Annotated[object, request_body]) -> object: ...

    @put_request("/products/{id}")
    async def replace(self, id: Annotated[int, path_variable("id")], product: Annotated[object, request_body]) -> object: ...

    @patch_request("/products/{id}")
    async def rename(self, id: Annotated[int, path_variable("id")], product: Annotated[object, request_body]) -> object: ...

    @delete_request("/products/{id}")
    async def remove(self, id: Annotated[int, path_variable("id")]) -> object: ...
