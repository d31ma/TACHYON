from typing import Protocol

from models import Product, Summary


@converter
class ProductConverter(Protocol):
    @Convert({"fields":[{"target":"kind","constant":"product"}]})
    def map(self, product: Product) -> Summary: ...

    def copy(self, product: Product) -> Product: ...
