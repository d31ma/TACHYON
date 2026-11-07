from typing import Annotated
from models import Item, ItemList


@request_route("/items")
@public
class ItemsController:
    @get_route
    def get_items(self) -> ItemList:
        return ItemList([])

    @post_route
    @response_status(201)
    def post_item(self, item: Annotated[Item, from_body]) -> Item:
        return item
