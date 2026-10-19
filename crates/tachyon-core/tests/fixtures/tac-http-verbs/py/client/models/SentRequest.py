from dataclasses import dataclass


@dataclass
class SentRequest:
    method: str
    path: str
    header: str
    item: str
