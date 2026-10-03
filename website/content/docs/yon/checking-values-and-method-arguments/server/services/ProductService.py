from typing import Annotated
from models import ProductInput

@service
class ProductService:
    def accept(self, input: Annotated[ProductInput, check]) -> ProductInput:
        return input
