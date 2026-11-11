from dataclasses import dataclass


@dataclass
class OrderInput:
    name: str
    level: int
