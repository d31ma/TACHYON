from typing import Annotated
from models import Item


@request_route("/items")
@public
class ItemsController:
    @get_route("/{id}")
    def get_item(self, id: Annotated[int, from_path]) -> Item:
        return Item(id)
