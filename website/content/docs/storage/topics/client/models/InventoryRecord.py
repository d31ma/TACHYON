from dataclasses import dataclass


@topic
@dataclass
class InventoryRecord:
    sku: str
    level: int
