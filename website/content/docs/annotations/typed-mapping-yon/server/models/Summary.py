from dataclasses import dataclass
from models import ShortDetail
@dataclass
class Summary:
    id: int
    name: str
    detail: ShortDetail
    items: list[ShortDetail]
    kind: str
