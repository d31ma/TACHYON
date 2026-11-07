from dataclasses import dataclass
from typing import Any


@dataclass
class CacheReport:
    first: Any
    repeat: Any
    written: Any
    refreshed: Any
