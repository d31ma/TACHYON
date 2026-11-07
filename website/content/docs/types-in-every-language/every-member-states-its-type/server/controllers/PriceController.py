from typing import Annotated
from models import Price


@request_route("/price")
@public
class PriceController:
    price_service: Annotated[PriceService, needs]

    @get_route
    def get_price(self) -> Price:
        return self.price_service.price_for(20, 3)
