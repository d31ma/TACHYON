from typing import Annotated
from models import Item


@request_route("/items")
@public
class ItemsController:
    @get_route("/{id}")
    def get_item(self, id: Annotated[str, from_path]) -> Item:
        return Item(id)

    @post_route
    @response_status(201)
    def post_item(self, item: Annotated[Item, from_body]) -> Item:
        return item
