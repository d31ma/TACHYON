from typing import Annotated

@service
class ProductService:
    def accept(self, input: Annotated[ProductInput, check]) -> ProductInput:
        return input
