from dataclasses import dataclass


@dataclass
class PlacedOrder:
    status: str
    total: int
