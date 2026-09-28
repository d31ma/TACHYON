from dataclasses import dataclass


@dataclass
class Price:
    amount: int
    currency: str
