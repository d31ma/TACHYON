from typing import Annotated


@service
class CatalogService:
    converter: Annotated[ProductConverter, needs]
    title: str = 'Not mapped.'

    def load(self) -> None:
        product = {'detail': {'id': 7}, 'name': 'Anvil', 'extra': 'dropped'}
        summary = self.converter.to_summary(product)
        self.title = f"{summary['kind']} {summary['productId']}: {summary['title']}"
