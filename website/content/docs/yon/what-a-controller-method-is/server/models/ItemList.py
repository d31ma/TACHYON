from dataclasses import dataclass

from models import Item


@dataclass
class ItemList:
    items: list[Item]
