from dataclasses import dataclass
from typing import Any


@dataclass
class LedgerReport:
    settled: Any
    commands: Any
