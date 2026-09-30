from typing import Protocol
from models import Product, Summary
@Converter
class ProductConverter(Protocol):
    @Convert({"fields":[{"target":"kind","constant":"product"}]})
    def map(self, product: Product) -> Summary: ...
