from typing import Annotated

class ValidationService:
    def accept(self, value: Annotated[ProductInput, check]):
        return value

