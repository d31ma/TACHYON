from typing import Annotated
from models import Money


@request_route("/money")
@public
class MoneyController:
    money: Annotated[MoneyService, needs]
    @get_route
    def get_money(self) -> Money:
        return Money(self.money.amount())
