from dataclasses import dataclass


@dataclass
class TaxAnswer:
    rate: int
    total: int
    extra: int
