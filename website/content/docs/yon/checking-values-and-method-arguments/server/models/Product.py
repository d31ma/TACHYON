from dataclasses import dataclass


@dataclass
class Product:
    name: str
    sku: str
    level: int
