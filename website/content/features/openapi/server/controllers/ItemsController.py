from typing import Annotated
from models import Item


@request_route("/items")
@public
class ItemsController:
    @get_route("/{id}")
    def get_item(self, id: Annotated[int, from_path]) -> Item:
        return Item("anvil", id)

    @patch_route("/{id}")
    def patch_item(self, id: Annotated[int, from_path], item: Annotated[Item, from_body]) -> Item:
        return Item(item.name, id)
