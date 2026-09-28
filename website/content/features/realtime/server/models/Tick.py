from dataclasses import dataclass


@dataclass
class Tick:
    sku: str
    level: int
