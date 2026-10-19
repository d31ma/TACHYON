from typing import Annotated

from models import Detail, Product


@service
class CatalogService:
    product_converter: Annotated[ProductConverter, needs]
    result: str = ''

    def load(self) -> None:
        detail = Detail('detail', 'detail-private')
        product = self.product_converter.copy(Product(7, 'Anvil', detail, [detail], 'product-private'))
        summary = self.product_converter.map(product)
        self.result = f'{summary.kind} {summary.id}: {summary.name}, {summary.detail.name}, {len(summary.items)}'
