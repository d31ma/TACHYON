from typing import Annotated

@http_client
class CatalogClient:
    behaviors: object

    @get_request("/products")
    async def all(self, page: Annotated[object, request_param("page")]) -> None:
        pass

    @post_request("/products")
    async def add(self, product: Annotated[object, request_body]) -> None:
        pass

    @get_request("/products")
    async def first(self) -> None:
        return {'query': {'page': 1}}

    @on_start
    async def initialize(self):
        self.behaviors = await Behaviors.create(CatalogSetup, {
            'partition': 'catalog',
            'signal': BehaviorSignal(),
        })
