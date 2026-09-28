from dataclasses import dataclass


@dataclass
class Ticket:
    sku: str
    level: int
