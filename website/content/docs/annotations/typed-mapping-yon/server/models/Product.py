from dataclasses import dataclass
from models import Detail
@dataclass
class Product:
    id: int
    name: str
    detail: Detail
    items: list[Detail]
    secret: str
