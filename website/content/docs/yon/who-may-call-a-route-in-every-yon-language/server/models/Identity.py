from dataclasses import dataclass


@dataclass
class Identity:
    user: str
    roles: list[str]
