from dataclasses import dataclass


@topic
@public
@dataclass
class InventoryRecord:
    sku: str
    level: int
