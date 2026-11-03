from dataclasses import dataclass
from typing import Any


@dataclass
class Forecast:
    summary: Any
    forecast: Any
