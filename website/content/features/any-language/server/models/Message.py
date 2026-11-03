from dataclasses import dataclass


@dataclass
class Message:
    name: str
    level: int
